use std::alloc::System;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::mem::take;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};
use std::vec::Vec;

use lib::config::{AutoRestart, Config, ProgramConfig};
use lib::process_status::{ProcessState, ProcessStatus};

use nix::unistd::Pid;

use super::process::*;

pub struct Supervisor {
    config: Config,
    processes: HashMap<String, Process>,
    trashes: Vec<Process>,
}

impl Supervisor {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let mut processes = HashMap::new();
        let trashes = Vec::new();

        for (_, v) in config.programs.iter() {
            for index in 0..v.numprocs {
                let process = Process::new(v, index)?;
                processes.insert(process.get_name(), process);
            }
        }

        Ok(Supervisor {
            config,
            processes,
            trashes,
        })
    }

    fn garbage_collect(&mut self) {
        let mut indexs: Vec<usize> = Default::default();
        for (i, r) in self.trashes.iter_mut().enumerate() {
            r.run().expect("");
            if r.is_stopped() {
                indexs.push(i);
            }
        }
        for i in indexs {
            self.trashes.remove(i);
        }
    }

    pub fn supervise(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (_, process) in self.processes.iter_mut() {
            process.run()?;
        }
        self.garbage_collect();
        Ok(())
    }

    fn remove_process(&mut self, process_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut process = self.processes.remove(process_name).unwrap();
        process.stop()?;
        self.trashes.push(process);
        Ok(())
    }

    fn add_process(
        &mut self,
        conf: &ProgramConfig,
        index: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let process = Process::new(conf, index)?;
        self.processes.insert(process.get_name(), process);
        Ok(())
    }

    // pub fn status(&self, names: Vec<String>) -> Result<Vec<ProcessStatus>, rpc_error::Error> {}
    pub fn update(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let next_conf = Config::from(file_path)?;
        let next_list = next_conf.process_list();
        let prev_list = self.config.process_list();
        let mut turn_off = &prev_list - &next_list;
        let mut turn_on = &next_list - &prev_list;
        let keep_or_restart = prev_list.intersection(&next_list);

        for process_id in keep_or_restart {
            let next = next_conf.programs.get(process_id.name.as_str()).unwrap();
            let prev = self.config.programs.get(process_id.name.as_str()).unwrap();

            if prev.diff(next) {
                turn_off.insert(process_id.clone());
                turn_on.insert(process_id.clone());
            }
        }

        for process_id in turn_off {
            self.remove_process(process_id.name.as_str())?;
        }

        for process_id in turn_on {
            let program_conf = next_conf.programs.get(process_id.name.as_str()).unwrap();
            self.add_process(program_conf, process_id.index)?;
        }

        self.config = next_conf;
        Ok(())
    }
}
