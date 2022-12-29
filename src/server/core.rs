use std::alloc::System;
use std::fmt::Display;
use std::vec::Vec;
use std::thread::sleep;
use std::mem::take;
use std::collections::HashMap;
use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::time::{Instant, SystemTime, Duration};

use lib::config::{AutoRestart, Config, ProgramConfig};
use lib::output_message::OutputMessage;
use lib::process_status::{ProcessState, ProcessStatus};
use lib::rpc_error;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

fn test() {
    let k = Command::new("abcd").output().expect("cde");

    let g = Instant::now();
}

struct Supervisor {
    config: Config,
    active_list: HashMap<String, Process>,
    inactive_list: HashMap<String, Process>,
}

impl Supervisor {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let mut active = HashMap::new();
        let mut inactive = HashMap::new();

        for (k, v) in config.programs.iter() {
            let mut process = Process::new(v)?;
            if v.autostart {
                process.start()?;
                active.insert(k.clone(), process);
            } else {
                inactive.insert(k.clone(), process);
            }
        }

        Ok(Supervisor {
            config,
            active_list: active,
            inactive_list: inactive,
        })
    }

    fn check_alive(
        &self,
        process: &mut Process,
        config: &ProgramConfig,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut running_process = process.process.as_mut().unwrap();

        match &process.state {
            ProcessState::Starting => {
                match running_process.try_wait() {
                    // died
                    Ok(Some(_)) => {
                        process.state = ProcessState::Backoff;
                        process.current_try += 1;
                        return Ok(true);
                    }

                    // alive
                    Ok(None) => {
                        if process.started_at.unwrap().elapsed().as_secs() > config.startsecs {
                            process.state = ProcessState::Running;
                        }
                        return Ok(true);
                    }
                    Err(e) => return Err(Box::new(e)),
                }
            }
            ProcessState::Running => {
                match running_process.try_wait() {
                    // died
                    Ok(Some(status)) => {
                        process.state = ProcessState::Exited;

                        match config.autorestart {
                            AutoRestart::Unexpected => {
                                match status.code() {
                                    Some(code) => {
                                        // expected code
                                        if config.exitcodes.contains(&code) {
                                            process.exited_at = Some(SystemTime::now());
                                            return Ok(false);
                                        }
                                        // unexpected code
                                        else {
                                            process.restart();
                                            return Ok(true);
                                        }
                                    }

                                    // got signal
                                    None => {
                                        process.restart();
                                        return Ok(true);
                                    }
                                }
                            }

                            AutoRestart::Always => {
                                process.restart();
                                return Ok(true);
                            }

                            AutoRestart::Never => {
                                process.exited_at = Some(SystemTime::now());
                                return Ok(false);
                            }
                        }
                    }

                    // alive
                    Ok(None) => {
                        return Ok(true);
                    }
                    Err(e) => return Err(Box::new(e)),
                }
            }
            ProcessState::Backoff => {
                if config.startretries < process.current_try {
                    process.state = ProcessState::Fatal;
                    return Ok(false);
                } else {
                    process.start();
                    return Ok(true);
                }
            }
            otherwise => panic!("unexpected state: {}", otherwise),
        };
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        
        loop {
            let mut active_list: HashMap<String, Process> = HashMap::new();
            let mut inactive_list: HashMap<String, Process> = HashMap::new();
            
            inactive_list = take(&mut self.inactive_list);
            active_list = take(&mut self.active_list);
            for ((whoami, process), (_, config)) in active_list.iter_mut().zip(self.config.programs.iter())
            {
                if !self.check_alive(process, config)? {
                    self.inactive_list.insert(whoami.to_string(), self.active_list.remove(whoami).unwrap());
                }
            }
            self.inactive_list = take(&mut inactive_list);
            self.active_list = take(&mut active_list);

            sleep(Duration::from_millis(500));
        }
    }
}

struct Process {
    pub state: ProcessState,
    command: Command,
    process: Option<Child>,
    current_try: u32,
    started_at: Option<Instant>,
    exited_at: Option<SystemTime>,
    exit_status: Option<u32>,
}

impl Process {
    pub fn new(conf: &ProgramConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let stdout = File::create(&conf.stdout_logfile)?;
        let stderr = File::create(&conf.stderr_logfile)?;

        let mut child = Command::new(&conf.command[0]);

        child
            .args(&conf.command[1..])
            .envs(&conf.environment)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr);

        Ok(Process {
            command: child,
            process: None,
            state: ProcessState::Stopped,
            current_try: 0,
            started_at: None,
            exited_at: None,
            exit_status: None,
        })
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.process = Some(self.command.spawn()?);
        self.state = ProcessState::Starting;
        self.started_at = Some(Instant::now());
        Ok(())
    }

    pub fn restart(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.start()?;
        self.current_try = 0;
        self.exited_at = None;
        self.exit_status = None;
        Ok(())
    }

    pub fn stop(&mut self, whoami: &str, signal: Signal) -> Result<(), rpc_error::Error> {
        if !self.state.stopable() {
            return Err(rpc_error::Error::ProcessAlreadyStarted(whoami.to_string()));
        }

        signal::kill(Pid::from_raw(self.process.as_ref().unwrap().id() as i32), signal)
            .map_err(|_| rpc_error::Error::ProcessNotFound(whoami.to_string()))
    }
}

pub fn shutdown() -> () {}

pub fn reload() -> () {}

pub fn healthCheck() -> () {}

// pub fn update<T: Display>() -> Result<Vec<OutputMessage>, T> {

// }

// pub fn status<T: Display>(programs: &Vec<String>) -> Result<Vec<ProcessStatus>, T> {

// }

// pub fn stop<T: Display>(program: &str) -> Result<OutputMessage, T> {

// }
// pub fn start<T: Display>(program: &str) -> Result<OutputMessage, T> {

// }
