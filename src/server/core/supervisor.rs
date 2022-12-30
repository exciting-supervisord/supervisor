use std::alloc::System;
use std::collections::HashMap;
use std::fmt::Display;
use std::mem::take;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};
use std::vec::Vec;

use lib::config::{AutoRestart, Config, ProgramConfig};
use lib::process_status::{ProcessState, ProcessStatus};

use nix::unistd::Pid;

use super::process::*;

struct Supervisor {
    config: Config,
    processes: HashMap<String, Process>,
}

impl Supervisor {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let mut processes = HashMap::new();

        for (k, v) in config.programs.iter() {
            let mut process = Process::new(v)?;
            if v.autostart {
                process.start()?;
            }
            processes.insert(k.clone(), process);
        }

        Ok(Supervisor { config, processes })
    }

    pub fn supervise(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            for (_, process) in self.processes.iter_mut() {
                process.run()?;
            }

            sleep(Duration::from_millis(500));
        }
    }

    // pub fn status(&self, names: Vec<String>) -> Result<Vec<ProcessStatus>, rpc_error::Error> {}

    pub fn update(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let next_conf = Config::from(file_path)?;
        let next_list = next_conf.program_list();
        let prev_conf = &self.config;
        let prev_list = prev_conf.program_list();

        // prev_list
        //     .iter()
        //     .filter(|x| !next_list.contains(*x))
        //     .for_each(|x| {
        //         self.active_list.remove(x).unwrap().stop()
        //         // self.inactive_list.remove(x);
        //     });

        // next_list
        //     .iter()
        //     .filter(|x| !prev_list.contains(*x))
        //     .for_each(|x| {
        //         // self.active_list.remove(x);
        //         // self.inactive_list.remove(x);
        //     });

        Ok(())
    }
}
