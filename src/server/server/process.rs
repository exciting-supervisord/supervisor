use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::time::{Instant, SystemTime};

use lib::config::{AutoRestart, ProcessConfig, ProgramConfig};
use lib::process_id::ProcessId;
use lib::process_status::{ProcessState, ProcessStatus};
use lib::response;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

pub trait IProcess {
    fn new(config: &ProgramConfig, index: u32) -> Result<Process, Box<dyn std::error::Error>>;
    fn start(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn is_stopped(&self) -> bool;
    fn get_status(&self) -> ProcessStatus;
    fn get_name(&self) -> String;
}

pub struct Process {
    pub exec: Option<Child>,
    pub command: Command,
    pub started_at: Option<Instant>,
    pub stop_at: Option<Instant>,
    pub exited_at: Option<SystemTime>, //?
    pub description: String,
    id: ProcessId,
    current_try: u32,
    state: ProcessState,
    exit_status: Option<i32>,
    conf: ProcessConfig,
}

impl IProcess for Process {
    fn get_name(&self) -> String {
        self.id.name.to_owned()
    }

    fn new(config: &ProgramConfig, index: u32) -> Result<Process, Box<dyn std::error::Error>> {
        let command = Process::new_command(config)?;
        let id = ProcessId {
            index,
            name: config.name.to_owned(),
        };
        let mut process = Process {
            id,
            command,
            exec: None,
            state: ProcessState::Stopped,
            current_try: 0,
            started_at: None,
            stop_at: None,
            exited_at: None,
            exit_status: None,
            description: String::from("Not started"),
            conf: ProcessConfig::from(config),
        };

        if config.autostart {
            process.start()?;
        }
        Ok(process)
    }

    fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.state.startable() {
            return Err(Box::new(response::Error::ProcessAlreadyStarted(
                self.id.name.to_owned(),
            )));
        }
        self.started_at = Some(Instant::now());
        self.state = ProcessState::Starting;
        self.spawn_process()
    }

    fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.state.stopable() {
            return Err(Box::new(response::Error::ProcessNotRunning(
                self.id.name.to_owned(),
            )));
        }
        self.state = ProcessState::Stopping;
        self.stop_at = Some(Instant::now());
        self.send_signal(self.conf.stopsignal)
    }

    fn is_stopped(&self) -> bool {
        self.state == ProcessState::Stopped
    }

    fn get_status(&self) -> ProcessStatus {
        ProcessStatus::new(
            self.id.name.to_owned(),
            self.state.clone(),
            self.description.to_string(),
        )
    }

    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    fn new_command(conf: &ProgramConfig) -> Result<Command, Box<dyn std::error::Error>> {
        println!("cmd = {}", conf.command[0]);
        println!("cmd = {}", &conf.stdout_logfile);
        println!("cmd = {}", &conf.stderr_logfile);
        
        let stdout = File::create(&conf.stdout_logfile)?;
        let stderr = File::create(&conf.stderr_logfile)?;

        let mut exec = Command::new(&conf.command[0]);

        exec.args(&conf.command[1..])
            .envs(&conf.environment)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr);
        Ok(exec)
    }

    fn spawn_process(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.exec = Some(self.command.spawn()?);
        Ok(())
    }

    fn send_signal(&mut self, signal: Signal) -> Result<(), Box<dyn std::error::Error>> {
        match signal::kill(
            Pid::from_raw(self.exec.as_ref().unwrap().id() as i32),
            signal,
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(Box::new(response::Error::ProcessNotFound(
                self.id.name.to_owned(),
            ))),
        }
    }

    fn autorestart(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.spawn_process()?;
        self.state = ProcessState::Starting;
        self.current_try = 0;
        self.exited_at = None;
        self.exit_status = None;
        Ok(())
    }

    fn is_process_alive(&mut self) -> bool {
        match self.exec.as_mut().unwrap().try_wait() {
            // alive
            Ok(None) => true,
            // died
            Ok(Some(status)) => {
                self.exit_status = status.code();
                false
            }
            Err(e) => panic!("there is no process: {e}"), // FIXME ㅎㅡㅁ..?
        }
    }

    fn starting(&mut self) {
        if self.is_process_alive() {
            if self.started_at.unwrap().elapsed().as_secs() > self.conf.startsecs {
                self.state = ProcessState::Running;
            }
        } else {
            self.state = ProcessState::Backoff;
            self.current_try += 1;
        }
    }

    fn running(&mut self) {
        if self.is_process_alive() {
            // set description using current time & started_at
            return;
        }
        self.state = ProcessState::Exited;
    }

    fn backoff(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.conf.startretries < self.current_try {
            self.state = ProcessState::Fatal;
        } else {
            self.spawn_process()?;
            self.state = ProcessState::Starting;
        }
        Ok(())
    }

    fn stopping(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_process_alive() {
            let interval = self.stop_at.unwrap().elapsed().as_secs();
            if interval > self.conf.stopwaitsecs {
                self.send_signal(Signal::SIGKILL)?;
            }
        } else {
            self.state = ProcessState::Stopped;
            // set description
        }
        Ok(())
    }

    fn stopped(&mut self) {
        // ?
    }

    fn exited(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let exitcodes = &self.conf.exitcodes;
        let autorestart = self.conf.autorestart;
        self.state = ProcessState::Exited;

        match autorestart {
            AutoRestart::Always => self.autorestart()?,
            AutoRestart::Never => self.exited_at = Some(SystemTime::now()),
            AutoRestart::Unexpected => {
                let none = self.exit_status.as_mut().and_then(|code| {
                    exitcodes
                        .contains(code)
                        .then(|| self.exited_at = Some(SystemTime::now()))
                });
                if let None = none {
                    self.autorestart()?;
                }
            }
        }
        Ok(())
    }

    fn fatal(&mut self) {
        // ?
    }
}