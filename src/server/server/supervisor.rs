use std::alloc::System;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::mem::take;
use std::rc::Rc;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};
use std::vec::Vec;

use lib::config::{AutoRestart, Config, ProgramConfig};
use lib::process_id::ProcessId;
use lib::process_status::{ProcessState, ProcessStatus};
use lib::response::{Error as RpcError, OutputMessage as RpcOutput, Response as RpcResponse};

use nix::unistd::Pid;

use crate::CONF_FILE;

use super::process::*;

pub struct Supervisor {
    config: Config,
    processes: HashMap<ProcessId, Process>,
    trashes: Vec<Process>,
}

impl Supervisor {
    pub fn sockfile(&self) -> &str {
        &self.config.general.sockfile
    }

    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let mut processes = HashMap::new();
        let trashes = Vec::new();

        for (_, v) in config.programs.iter() {
            for index in 0..v.numprocs {
                let process = Process::new(v, index)?;
                processes.insert(process.get_id(), process);
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

    fn remove_process(&mut self, process_id: &ProcessId) -> Result<(), RpcError> {
        let mut process = self.processes.remove(process_id).unwrap();
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
        self.processes.insert(process.get_id(), process);
        Ok(())
    }

    // pub fn status(&self, names: Vec<String>) -> Result<Vec<ProcessStatus>, rpc_error::Error> {}

    const CONF_FILE: &'static str = "./general.ini";

    fn affect(&mut self, next_conf: &Config) {
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
            self.remove_process(&process_id);
        }

        for process_id in turn_on {
            let program_conf = next_conf.programs.get(process_id.name.as_str()).unwrap();
            self.add_process(program_conf, process_id.index);
        }
    }

    pub fn update(&mut self, _: &Vec<String>) -> RpcResponse {
        let file_path = CONF_FILE; // FIXME

        let next_conf = match Config::from(file_path) {
            Ok(o) => o,
            Err(e) => return RpcResponse::from_err(RpcError::file_format(e.to_string().as_str())),
        };

        self.affect(&next_conf);
        self.config = next_conf;
        RpcResponse::from_output(RpcOutput::new("configuration", "updated"))
    }

    fn convert_to_process_ids(&self, names: &Vec<String>) -> Vec<ProcessId> {
        names
            .iter()
            .map(|x| {
                let (name, index) = x.split_once(":").expect("return Invalid argument"); // ? 클라이언트에서 처리하는게 맞을 지도?
                ProcessId {
                    name: name.to_owned(),
                    index: index.parse::<u32>().expect("parse fail"),
                }
            })
            .collect::<Vec<ProcessId>>()
    }

    fn try_order_once(&mut self, id: &ProcessId, order: &str) -> Result<RpcOutput, RpcError> {
        let runnings = self.config.process_list();

        if runnings.contains(id) == false {
            Err(RpcError::ProcessNotFound(id.name.to_owned()))
        } else {
            match order {
                "start" => match self.processes.get_mut(id).unwrap().start() {
                    Err(e) => Err(RpcError::Service(e.to_string())), // FIXME ?
                    Ok(_) => Ok(RpcOutput::new(id.name.as_str(), "started")),
                },
                "stop" => match self.processes.get_mut(id).unwrap().stop() {
                    Err(e) => Err(RpcError::Service(e.to_string())), // FIXME ?
                    Ok(_) => Ok(RpcOutput::new(id.name.as_str(), "stopped")),
                },
                _ => panic!("logic error"),
            }
        }
    }

    // Start(name) -> Vec<Respose = Result( OutputMessage, Error)>
    // where Error: ProcessNotFoundError + ProcessAlreadyStartedError
    pub fn start(&mut self, names: Vec<String>) -> RpcResponse {
        let inputs = self.convert_to_process_ids(&names);
        let runnings = self.config.process_list();

        inputs
            .iter()
            .map(|id| self.try_order_once(id, "start"))
            .collect::<RpcResponse>()
    }

    // Stop(name) -> Result( OutputMessage, Error)
    // where Error: ProcessNotFoundError + ProcessNotRunningError
    pub fn stop(&mut self, names: Vec<String>) -> RpcResponse {
        let inputs = self.convert_to_process_ids(&names);
        let runnings = self.config.process_list();

        inputs
            .iter()
            .map(|id| self.try_order_once(id, "stop"))
            .collect::<RpcResponse>()
    }

    // Reload() -> ()
    pub fn reload() {}

    //     Shutdown() -> ()
    pub fn shutdown() {}

    // Status(Vec<name>) -> Result( Vec<ProcessStatus>, Error)
    // where Error: ServiceError + ProcessNotFoundError
    pub fn status() {}
}
