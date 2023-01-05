mod process;

use std::collections::HashMap;
use std::mem;
use std::vec::Vec;

use lib::config::{Config, ProgramConfig};
use lib::process_id::ProcessId;
use lib::process_status::ProcessStatus;
use lib::response::{
    Action, Error as RpcError, OutputMessage as RpcOutput, Response as RpcResponse,
};
use lib::logger::LOG;

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
        let mut processes = HashMap::new();

        for (_, v) in config.programs.iter() {
            for index in 0..v.numprocs {
                let process = Process::new(v, index)?;
                processes.insert(process.get_id(), process);
            }
        }
        Ok(Supervisor {
            file_path: file_path.to_owned(),
            config,
            processes,
            trashes: Vec::new(),
        })
    }

    pub fn supervise(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (_, process) in self.processes.iter_mut() {
            process.run()?;
        }
        self.garbage_collect();
        Ok(())
    }

    fn garbage_collect(&mut self) {
        self.trashes.iter_mut().for_each(|p| p.run().expect("")); // FIXME
        self.trashes.retain(|p| !p.is_stopped());
    }

    pub fn start(&mut self, names: Vec<String>) -> RpcResponse {
        LOG.info(&format!("handle request - start, names={:?}", names));
        let inputs = self.convert_to_process_ids(&names);

        let act = inputs
            .iter()
            // .map(|id| self.try_process_operation(id, |proc| proc.start()))
            .map(|id| self.try_process_operation(id, Process::start))
            .collect::<Action>();
        RpcResponse::Action(act)
    }

    pub fn stop(&mut self, names: Vec<String>) -> RpcResponse {
        LOG.info(&format!("handle request - stop, names={:?}", names));
        let inputs = self.convert_to_process_ids(&names);

        let act = inputs
            .iter()
            .map(|id| self.try_process_operation(id, Process::stop))
            .collect::<Action>();
        RpcResponse::Action(act)
    }

    // Reload() -> ()
    pub fn reload(&mut self, _: Vec<String>) -> RpcResponse {
        LOG.info("handle request - reload");
        self.cleanup_processes();

        let config = mem::take(&mut self.config); // TODO 생각해보기: 이게 맞나..?
        let turn_on = config.process_list();

        for process_id in turn_on {
            let program_conf = config.programs.get(process_id.name.as_str()).unwrap();
            self.add_process(program_conf, process_id.seq);
        }
        self.config = config;
        RpcResponse::from_output(RpcOutput::new("taskmaster", "reload"))
    }

    //     Shutdown() -> ()
    pub fn shutdown(&mut self, _: Vec<String>) -> RpcResponse {
        LOG.info("handle request - shutdown");
        self.cleanup_processes();
        RpcResponse::from_output(RpcOutput::new("taskmaster", "shutdown"))
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
        seq: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let process = Process::new(conf, seq)?;
        self.processes.insert(process.get_id(), process);
        Ok(())
    }

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

    pub fn update(&mut self, _: Vec<String>) -> RpcResponse {
        LOG.info("handle request - update");
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

    fn try_process_operation(
        &mut self,
        id: &ProcessId,
        operation: fn(id: &mut Process) -> Result<RpcOutput, RpcError>,
    ) -> Result<RpcOutput, RpcError> {
        if !self.config.process_list().contains(id) {
            return Err(RpcError::ProcessNotFound(id.name.to_owned()));
        }

        let process = self
            .processes
            .get_mut(id)
            .expect("running process must in processes");
        operation(process)
    }

    fn cleanup_processes(&mut self) {
        let keys: Vec<ProcessId> = self.processes.iter().map(|(k, _)| k.to_owned()).collect();

        for key in keys {
            self.remove_process(&key);
        }

        // TODO 고민: 선 응답 후 처리?
        while self.trashes.len() != 0 {
            self.garbage_collect();
        }
    }

    // Status(Vec<name>) -> Result( Vec<ProcessStatus>, Error)
    // where Error: ServiceError + ProcessNotFoundError
    pub fn status(&self, words: Vec<String>) -> RpcResponse {
        LOG.info("handle request - status");
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
