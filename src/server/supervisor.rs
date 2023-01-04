mod process;

use std::collections::HashMap;
use std::vec::Vec;

use lib::config::{Config, ProgramConfig};
use lib::process_id::ProcessId;
use lib::process_status::ProcessStatus;
use lib::response::{
    Action, Error as RpcError, OutputMessage as RpcOutput, Response as RpcResponse,
};

use process::*;

pub struct Supervisor {
    file_path: String,
    config: Config,
    processes: HashMap<ProcessId, Process>,
    trashes: Vec<Process>,
}

impl Supervisor {
    pub fn sockfile(&self) -> &str {
        &self.config.general.sockfile
    }

    pub fn new(file_path: &str, config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let file_path = file_path.to_owned();
        let mut processes = HashMap::new();
        let trashes = Vec::new();

        for (_, v) in config.programs.iter() {
            for index in 0..v.numprocs {
                let process = Process::new(v, index)?;
                processes.insert(process.get_id(), process);
            }
        }

        Ok(Supervisor {
            file_path,
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
            self.add_process(program_conf, process_id.seq);
        }
    }

    pub fn update(&mut self, _: &Vec<String>) -> RpcResponse {
        let next_conf = match Config::from(&self.file_path) {
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
                let (name, seq) = x.split_once(":").expect("return Invalid argument"); // ? 클라이언트에서 처리하는게 맞을 지도?
                ProcessId {
                    name: name.to_owned(),
                    seq: seq.parse::<u32>().expect("parse fail"),
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
                    Err(e) => Err(e), // FIXME 추상화....!
                    Ok(_) => Ok(RpcOutput::new(id.name.as_str(), "started")),
                },
                "stop" => match self.processes.get_mut(id).unwrap().stop() {
                    Err(e) => Err(e),
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

        let act = inputs
            .iter()
            .map(|id| self.try_order_once(id, "start"))
            .collect::<Action>();
        RpcResponse::Action(act)
    }

    // Stop(name) -> Result( OutputMessage, Error)
    // where Error: ProcessNotFoundError + ProcessNotRunningError
    pub fn stop(&mut self, names: Vec<String>) -> RpcResponse {
        let inputs = self.convert_to_process_ids(&names);

        let act = inputs
            .iter()
            .map(|id| self.try_order_once(id, "stop"))
            .collect::<Action>();
        RpcResponse::Action(act)
    }

    // Reload() -> ()
    // pub fn reload() {}

    //     Shutdown() -> ()
    // pub fn shutdown() {}

    // Status(Vec<name>) -> Result( Vec<ProcessStatus>, Error)
    // where Error: ServiceError + ProcessNotFoundError
    pub fn status(&self, words: Vec<String>) -> RpcResponse {
        if words.contains(&String::from("all")) || words.len() == 0 {
            let v: Vec<ProcessStatus> = self
                .processes
                .iter()
                .map(|(_, proc)| proc.get_status())
                .collect();
            return RpcResponse::Status(v);
        }

        let ids: Vec<ProcessId> = words
            .iter()
            .map(|id| {
                let (name, seq) = id.split_once(":").unwrap();
                let seq = seq.parse::<u32>().unwrap();
                ProcessId::new(name.to_owned(), seq)
            })
            .collect();

        let v: Vec<ProcessStatus> = ids
            .iter()
            .map(|id| self.processes.get(id).unwrap().get_status())
            .collect();
        RpcResponse::Status(v)
    }
}
