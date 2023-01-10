use core::ffi::c_char;
use std::env::set_current_dir;
use std::io::ErrorKind;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

use lib::config::{AutoRestart, ProcessConfig, ProgramConfig};
use lib::logger::Logger;
use lib::logger::LOG;
use lib::process_id::ProcessId;
use lib::process_status::{ProcessState, ProcessStatus};
use lib::response::{Error as RpcError, OutputMessage as RpcOutput};

use nix::libc::{dup2, getpwnam, getuid, open};
use nix::sys::signal::{self, Signal};
use nix::sys::stat::{umask, Mode};
use nix::unistd::{setuid, Pid, Uid};

use libc::{O_CREAT, O_TRUNC, O_WRONLY};

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
    id: ProcessId,
    current_try: u32,
    state: ProcessState,
    exit_status: Option<i32>,
    conf: ProcessConfig,
    start_at: Option<Instant>,
    stop_at: Option<Instant>,
    description: String,
}

impl IProcess for Process {
    fn get_id(&self) -> ProcessId {
        ProcessId::new(self.id.name.to_owned(), self.id.seq)
    }

    fn new(config: &ProgramConfig, index: u32) -> Result<Process, RpcError> {
        let command = Process::new_command(config)?;
        let id = ProcessId::new(config.name.to_owned(), index);
        let process = Process {
            id,
            command,
            proc: None,
            state: ProcessState::Stopped,
            current_try: 1,
            start_at: None,
            stop_at: None,
            exit_status: None,
            description: String::from(INIT_DESCRIPTION),
            conf: ProcessConfig::from(config),
        };
        Ok(process)
    }

    fn start(&mut self) -> Result<RpcOutput, RpcError> {
        let id = self.id.to_string();

        if !self.state.startable() {
            return Err(RpcError::ProcessAlreadyStarted(id));
        }
        self.start_process()
            .map(|_| RpcOutput::new(id.as_str(), "started"))
    }

    fn stop(&mut self) -> Result<RpcOutput, RpcError> {
        let id = self.id.to_string();

        if !self.state.stopable() {
            return Err(RpcError::ProcessNotRunning(id));
        }
        self.stop_at = Some(Instant::now());
        self.goto(ProcessState::Stopping, format!(""));
        self.send_signal(self.conf.stopsignal)
            .map(|_| RpcOutput::new(id.as_str(), "stopping"))
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
        let mut stdout_path = conf.stdout_logfile.to_owned();
        let mut stderr_path = conf.stderr_logfile.to_owned();

        stderr_path.push('\0');
        stdout_path.push('\0');

        let v_uid = Process::get_uid(&conf.user);
        let v_umask = conf.umask.unwrap_or(0o022);
        let directory = conf.directory.clone();

        let mut cmd = Command::new(&conf.command[0]);

        cmd.args(&conf.command[1..])
            .envs(&conf.environment)
            .stdin(Stdio::null());

        unsafe {
            cmd.pre_exec(move || {
                setuid(Uid::from_raw(v_uid))?;
                umask(Mode::from_bits(v_umask).unwrap());
                let stderr_ptr = stderr_path.as_ptr() as *const c_char;
                let stdout_ptr = stdout_path.as_ptr() as *const c_char;
                let stderr = open(stderr_ptr, O_WRONLY | O_TRUNC | O_CREAT, 0o777);
                let stdout = open(stdout_ptr, O_WRONLY | O_TRUNC | O_CREAT, 0o777);
                if stdout < 0 || stderr < 0 || dup2(stderr, 2) < 0 || dup2(stdout, 1) < 0 {
                    LOG.crit(&format!(
                        "setting logfile failed: {stdout_path}: {stdout}, {stderr_path}: {stderr}"
                    ));
                    return Err(std::io::Error::new(
                        ErrorKind::Other,
                        format!("can not spawn"),
                    ));
                }
                set_current_dir(directory.to_owned())
            });
        }
        Ok(cmd)
    }

    fn get_uid(user_name: &Option<String>) -> u32 {
        if let None = user_name {
            return unsafe { getuid() };
        }
        let mut user_name = user_name.as_ref().unwrap().to_string();
        user_name.push('\0');
        unsafe {
            let name_ptr = user_name.as_ptr() as *const i8;
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

    fn start_process(&mut self) -> Result<(), RpcError> {
        self.spawn_process()?;
        self.start_at = Some(Instant::now());
        self.goto(ProcessState::Starting, format!(""));
        Ok(())
    }

    fn spawn_process(&mut self) -> Result<(), RpcError> {
        let proc = self.command.spawn();

        if let Err(e) = proc {
            self.goto(ProcessState::Fatal, format!("spawn failed - error={}", e));
            return Err(RpcError::spawn(e.to_string().as_str()));
        }

        self.proc = Some(proc.unwrap());
        Ok(())
    }

    fn send_signal(&mut self, signal: Signal) -> Result<(), RpcError> {
        let proc = self.proc.as_ref().unwrap();
        let pid = Pid::from_raw(proc.id() as i32);
        LOG.info(&format!("send {signal} to [{}]", self.id));
        signal::kill(pid, signal).map_err(|_| RpcError::ProcessNotFound(self.id.name.to_owned()))
    }

    fn autorestart(&mut self) -> Result<(), RpcError> {
        self.start_process()?;
        self.current_try = 1;
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
            let running_millis = self.start_at.unwrap().elapsed().as_millis() as u64;
            if running_millis > self.conf.startsecs * 1000 - lib::EVENT_LOOP_TIME {
                self.goto(
                    ProcessState::Running,
                    format!("pid {}, uptime 0:00:00", self.proc.as_ref().unwrap().id(),),
                );
            }
        } else {
            self.goto(ProcessState::Backoff, format!("Exited too quickly."));
            self.current_try += 1;
        }
    }

    fn running(&mut self) {
        if self.is_process_alive() {
            let running_secs = self.start_at.unwrap().elapsed().as_secs();
            let hours = running_secs / 3600;
            let mins = (running_secs % 3600) / 60;
            let secs = running_secs % 60;
            let time = format!("{}:{:02}:{:02}", hours, mins, secs);
            self.description = format!("pid {}, uptime {}", self.proc.as_ref().unwrap().id(), time);
        } else {
            let unexpected = match self.exit_status {
                Some(ref code) if !self.conf.exitcodes.contains(code) => {
                    String::from(" unexpected")
                }
                _ => String::from(""),
            };
            let description = Logger::get_formated_timestamp() + &unexpected;
            self.goto(ProcessState::Exited, description);
        }
    }

    fn backoff(&mut self) -> Result<(), RpcError> {
        if self.conf.startretries < self.current_try {
            self.goto(ProcessState::Fatal, self.description.clone());
        } else {
            self.start_process()?;
        }
        Ok(())
    }

    fn stopping(&mut self) -> Result<(), RpcError> {
        if self.is_process_alive() {
            let interval = self.stop_at.unwrap().elapsed().as_millis() as u64;
            if interval > self.conf.stopwaitsecs * 1000 - lib::EVENT_LOOP_TIME {
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
        LOG.info(&format!(
            "[{}] state goes to {}",
            self.id,
            state.to_string()
        ));

        self.state = state;
        self.description = description;
    }
}
