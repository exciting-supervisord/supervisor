use std::env::set_current_dir;
use std::fs::File;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

use lib::config::{AutoRestart, ProcessConfig, ProgramConfig};
use lib::logger::Logger;
use lib::logger::LOG;
use lib::process_id::ProcessId;
use lib::process_status::{ProcessState, ProcessStatus};
use lib::response::{Error as RpcError, OutputMessage as RpcOutput};

use nix::libc::{getpwnam, getuid};
use nix::sys::signal::{self, Signal};
use nix::sys::stat::{umask, Mode};
use nix::unistd::Pid;

const INIT_DESCRIPTION: &'static str = "Not started";

pub trait IProcess {
    fn new(config: &ProgramConfig, index: u32) -> Result<Process, RpcError>;
    fn start(&mut self) -> Result<RpcOutput, RpcError>;
    fn stop(&mut self) -> Result<RpcOutput, RpcError>;
    fn run(&mut self) -> Result<(), RpcError>;
    fn is_stopped(&self) -> bool;
    fn get_status(&self) -> ProcessStatus;
    fn get_id(&self) -> ProcessId;
}

pub struct Process {
    pub proc: Option<Child>,
    pub command: Command,
    pub started_at: Option<Instant>,
    pub stop_at: Option<Instant>,
    pub description: String,
    id: ProcessId,
    current_try: u32,
    state: ProcessState,
    exit_status: Option<i32>,
    conf: ProcessConfig,
}

impl IProcess for Process {
    fn get_id(&self) -> ProcessId {
        ProcessId::new(self.id.name.to_owned(), self.id.seq)
    }

    fn new(config: &ProgramConfig, index: u32) -> Result<Process, RpcError> {
        let command = Process::new_command(config)?;
        println!("{}", config.name);
        let id = ProcessId::new(config.name.to_owned(), index);
        let mut process = Process {
            id,
            command,
            proc: None,
            state: ProcessState::Stopped,
            current_try: 1,
            started_at: None,
            stop_at: None,
            exit_status: None,
            description: String::from(INIT_DESCRIPTION),
            conf: ProcessConfig::from(config),
        };

        if config.autostart {
            process.start()?;
        }
        Ok(process)
    }

    fn start(&mut self) -> Result<RpcOutput, RpcError> {
        let name = self.id.to_string();

        if !self.state.startable() {
            return Err(RpcError::ProcessAlreadyStarted(name));
        }
        self.started_at = Some(Instant::now());
        self.goto(ProcessState::Starting, format!(""));
        self.spawn_process()
            .map(|_| RpcOutput::new(name.as_str(), "started"))
    }

    fn stop(&mut self) -> Result<RpcOutput, RpcError> {
        let name = self.id.to_string();

        if !self.state.stopable() {
            return Err(RpcError::ProcessNotRunning(name));
        }
        self.goto(ProcessState::Stopping, format!(""));
        self.stop_at = Some(Instant::now());
        self.send_signal(self.conf.stopsignal)
            .map(|_| RpcOutput::new(name.as_str(), ""))
    }

    fn is_stopped(&self) -> bool {
        self.state == ProcessState::Stopped
    }

    fn get_status(&self) -> ProcessStatus {
        ProcessStatus::new(
            self.id.name.to_owned(),
            self.id.seq,
            self.state.clone(),
            self.description.to_string(),
        )
    }

    fn run(&mut self) -> Result<(), RpcError> {
        match self.state {
            ProcessState::Starting => self.starting(),
            ProcessState::Running => self.running(),
            ProcessState::Backoff => self.backoff()?,
            ProcessState::Stopping => self.stopping()?,
            ProcessState::Stopped => self.stopped(),
            ProcessState::Exited => self.exited()?,
            ProcessState::Fatal => self.fatal(),
            ProcessState::Unknown => panic!("invalid process state"),
        }
        Ok(())
    }
}

impl Process {
    fn new_command(conf: &ProgramConfig) -> Result<Command, RpcError> {
        let stdout_path = conf.stdout_logfile.to_owned();
        let stderr_path = conf.stderr_logfile.to_owned();

        let stdout =
            File::create(&stdout_path).map_err(|e| RpcError::file_open(e.to_string().as_str()))?;
        let stderr =
            File::create(&stderr_path).map_err(|e| RpcError::file_open(e.to_string().as_str()))?;

        let v_uid = Process::get_uid(&conf.user);
        let v_umask = conf.umask.unwrap_or(0o022);
        let directory = conf.directory.clone();

        let mut cmd = Command::new(&conf.command[0]);

        cmd.args(&conf.command[1..])
            .envs(&conf.environment)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .uid(v_uid);

        unsafe {
            cmd.pre_exec(move || {
                File::create(stderr_path.to_owned())?;
                File::create(stdout_path.to_owned())?;
                umask(Mode::from_bits(v_umask).unwrap());
                set_current_dir(directory.to_owned())
            });
        }
        Ok(cmd)
    }

    fn get_uid(user_name: &Option<String>) -> u32 {
        if let None = user_name {
            return unsafe { getuid() };
        }
        let user_name = user_name.as_ref().unwrap();
        let name_ptr = user_name.as_ptr() as *const i8;
        unsafe {
            let passwd = getpwnam(name_ptr);
            if passwd.is_null() {
                let msg = format!("there is no user named {user_name}. the process uid will be set to taskmasterd's.");
                LOG.warn(msg.as_str());
                getuid()
            } else {
                (*passwd).pw_uid
            }
        }
    }

    fn spawn_process(&mut self) -> Result<(), RpcError> {
        let proc = self.command.spawn();

        if let Err(e) = proc {
            self.goto(ProcessState::Stopped, format!("spawn failed - error={}", e));
            return Err(RpcError::spawn(e.to_string().as_str()));
        }

        self.proc = Some(proc.unwrap());
        Ok(())
    }

    fn send_signal(&mut self, signal: Signal) -> Result<(), RpcError> {
        let proc = self.proc.as_ref().unwrap();
        let pid = Pid::from_raw(proc.id() as i32);

        signal::kill(pid, signal).map_err(|_| RpcError::ProcessNotFound(self.id.name.to_owned()))
    }

    fn autorestart(&mut self) -> Result<(), RpcError> {
        self.spawn_process()?;
        self.goto(ProcessState::Starting, format!(""));
        self.current_try = 0;
        self.exit_status = None;
        Ok(())
    }

    fn is_process_alive(&mut self) -> bool {
        match self.proc.as_mut().unwrap().try_wait() {
            // alive
            Ok(None) => true,
            // died
            Ok(Some(status)) => {
                self.exit_status = status.code();
                false
            }
            Err(e) => {
                LOG.crit(&format!("{e}"));
                true
            }
        }
    }

    fn starting(&mut self) {
        if self.is_process_alive() {
            if self.started_at.unwrap().elapsed().as_secs() > self.conf.startsecs {
                self.goto(
                    ProcessState::Running,
                    format!("pid {}, uptime 0:00:00", self.proc.as_ref().unwrap().id(),),
                );
            }
        } else {
            self.goto(
                ProcessState::Backoff,
                format!("Exited too quickly (process log may have details)"),
            );
            self.current_try += 1;
        }
    }

    fn running(&mut self) {
        if self.is_process_alive() {
            let time_u64 = self.started_at.unwrap().elapsed().as_secs();
            let hours = time_u64 / 3600;
            let mins = (time_u64 % 3600) / 60;
            let secs = time_u64 % 60;
            let time = format!("{}:{:02}:{:02}", hours, mins, secs);
            self.description = format!("pid {}, uptime {}", self.proc.as_ref().unwrap().id(), time);
            return;
        }
        self.goto(ProcessState::Exited, Logger::get_formated_timestamp());
    }

    fn backoff(&mut self) -> Result<(), RpcError> {
        if self.conf.startretries < self.current_try {
            self.state = ProcessState::Fatal;
        } else {
            self.spawn_process()?;
            self.goto(ProcessState::Starting, format!(""));
        }
        Ok(())
    }

    fn stopping(&mut self) -> Result<(), RpcError> {
        if self.is_process_alive() {
            let interval = self.stop_at.unwrap().elapsed().as_secs();
            if interval > self.conf.stopwaitsecs {
                self.send_signal(Signal::SIGKILL)?;
            }
        } else {
            self.goto(ProcessState::Stopped, Logger::get_formated_timestamp());
        }
        Ok(())
    }

    fn exited(&mut self) -> Result<(), RpcError> {
        let exitcodes = &self.conf.exitcodes;
        let autorestart = self.conf.autorestart;

        match autorestart {
            AutoRestart::Always => self.autorestart()?,
            AutoRestart::Unexpected => match self.exit_status {
                Some(ref code) if !exitcodes.contains(code) => {
                    self.autorestart()?;
                }
                _ => {}
            },
            AutoRestart::Never => {}
        }
        Ok(())
    }

    fn stopped(&mut self) {}

    fn fatal(&mut self) {}

    fn goto(&mut self, state: ProcessState, description: String) {
        self.state = state;
        self.description = description;
    }
}
