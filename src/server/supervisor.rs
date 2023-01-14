mod process;

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::vec::Vec;

use lib::config::Config;
use lib::logger::LOG;
use lib::process_id::{ProcessId, ToProcessIds};
use lib::process_status::ProcessStatus;
use lib::response::{CommandResult, Response as RpcResponse, RpcError, RpcOutput};

use super::control;
use process::*;

pub trait ISupervisor {
    fn sockfile(&self) -> &str;
    fn new(file_path: &str, config: Config) -> Result<Supervisor, Box<dyn std::error::Error>>;
    fn supervise(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn start(&mut self, names: Vec<String>) -> RpcResponse;
    fn stop(&mut self, names: Vec<String>) -> RpcResponse;
    fn restart(&mut self, names: Vec<String>) -> RpcResponse;
    fn update(&mut self, _: Vec<String>) -> RpcResponse;
    fn cleanup_processes(&mut self);
    fn status(&self, names: Vec<String>) -> RpcResponse;
    fn reload(&mut self, _: Vec<String>) -> RpcResponse;
    fn shutdown(&mut self, _: Vec<String>) -> RpcResponse;
}

pub struct Supervisor {
    file_path: String,
    config: Config,
    processes: HashMap<ProcessId, Process>,
    trashes: Vec<Process>,
}

impl ISupervisor for Supervisor {
    fn sockfile(&self) -> &str {
        &self.config.general.sockfile
    }

    fn new(file_path: &str, config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let mut supervisor = Supervisor {
            file_path: file_path.to_owned(),
            config,
            processes: HashMap::new(),
            trashes: Vec::new(),
        };

        for process_id in supervisor.config.process_list() {
            supervisor.add_process(&process_id)?;
        }
        Ok(supervisor)
    }

    fn supervise(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (_, process) in self.processes.iter_mut() {
            process.run()?;
        }
        self.garbage_collect();
        Ok(())
    }

    fn start(&mut self, names: Vec<String>) -> RpcResponse {
        LOG.info(&format!("handle request - start, names={:?}", names));
        let ids = names.to_process_ids();
        self.start_processes(ids.iter())
    }

    fn stop(&mut self, names: Vec<String>) -> RpcResponse {
        LOG.info(&format!("handle request - stop, names={:?}", names));
        let process_ids = names.to_process_ids();
        self.stop_processes(process_ids.iter())
    }

    fn restart(&mut self, names: Vec<String>) -> RpcResponse {
        LOG.info(&format!("handle request - stop, names={:?}", names));
        let ids = names.to_process_ids();

        let res1 = self.stop_processes(ids.iter());
        self.remove_processes(ids.iter());
        let res2 = self.revive_processes(ids.iter());
        res1 + res2
    }

    fn update(&mut self, _: Vec<String>) -> RpcResponse {
        LOG.info("handle request - update");
        let next_conf = match Config::from(&self.file_path) {
            Ok(o) => o,
            Err(e) => return RpcResponse::from_err(RpcError::file_format(e.to_string().as_str())),
        };

        self.affect(next_conf);
        RpcResponse::from_output(RpcOutput::new("configuration", "updated"))
    }

    fn cleanup_processes(&mut self) {
        let keys: Vec<ProcessId> = self.processes.iter().map(|(k, _)| k.to_owned()).collect();

        self.stop_processes(keys.iter());
        self.remove_processes(keys.iter());

        while self.trashes.len() != 0 {
            self.garbage_collect();
        }
    }

    fn status(&self, names: Vec<String>) -> RpcResponse {
        LOG.info("handle request - status");

        let ids = names.to_process_ids();
        let v: Vec<ProcessStatus> = ids
            .iter()
            .map(|id| self.processes.get(id).unwrap().get_status())
            .collect();
        RpcResponse::Status(v)
    }

    fn reload(&mut self, _: Vec<String>) -> RpcResponse {
        LOG.info("handle request - reload");
        self.cleanup_processes();

        let turn_on = self.config.process_list();
        for process_id in turn_on {
            self.add_process(&process_id).unwrap_or_default();
        }
        RpcResponse::from_output(RpcOutput::new("taskmasterd", "reload"))
    }

    fn shutdown(&mut self, _: Vec<String>) -> RpcResponse {
        LOG.info("handle request - shutdown");
        control::SHUTDOWN.store(true, Ordering::Relaxed);
        RpcResponse::from_output(RpcOutput::new("taskmasterd", "shutdown"))
    }
}

impl Supervisor {
    fn garbage_collect(&mut self) {
        self.trashes
            .iter_mut()
            .for_each(|p| p.run().unwrap_or_default());
        self.trashes.retain(|p| !p.is_stopped());
    }

    fn affect(&mut self, next_conf: Config) {
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

        self.stop_processes(turn_off.iter());
        self.remove_processes(turn_off.iter());

        self.config = next_conf;
        for process_id in turn_on {
            self.add_process(&process_id).unwrap_or_default();
        }
    }

    fn add_process(&mut self, process_id: &ProcessId) -> Result<(), RpcError> {
        let conf = self.config.programs.get(&process_id.name).unwrap();

        let mut process = Process::new(conf, process_id.seq)?;
        if conf.autostart {
            process.start()?;
        }
        self.processes.insert(process.get_id(), process);
        Ok(())
    }

    fn remove_processes<'a>(&mut self, process_ids: impl Iterator<Item = &'a ProcessId>) {
        process_ids.for_each(|id| {
            let process = self.processes.remove(id).unwrap();
            self.trashes.push(process);
        });
    }

    fn revive_processes<'a>(
        &mut self,
        process_ids: impl Iterator<Item = &'a ProcessId>,
    ) -> RpcResponse {
        let cmd_res = process_ids
            .map(|id| {
                self.config.programs.get(&id.name).map_or(
                    Err(RpcError::ProcessNotFound(id.to_string())),
                    |conf| {
                        let mut p = Process::new(conf, id.seq)?;
                        let ret = p.start()?;
                        self.processes.insert(p.get_id(), p);
                        Ok(ret)
                    },
                )
            })
            .collect::<Vec<CommandResult>>();
        RpcResponse::Command(cmd_res)
    }

    fn stop_processes<'a>(
        &mut self,
        process_ids: impl Iterator<Item = &'a ProcessId>,
    ) -> RpcResponse {
        let cmd_res = process_ids
            .map(|id| self.try_process_operation(id, Process::stop))
            .collect::<Vec<CommandResult>>();
        RpcResponse::Command(cmd_res)
    }

    fn start_processes<'a>(
        &mut self,
        process_ids: impl Iterator<Item = &'a ProcessId>,
    ) -> RpcResponse {
        let cmd_res = process_ids
            .map(|id| self.try_process_operation(id, Process::start))
            .collect::<Vec<CommandResult>>();
        RpcResponse::Command(cmd_res)
    }

    fn try_process_operation(
        &mut self,
        id: &ProcessId,
        operation: fn(id: &mut Process) -> Result<RpcOutput, RpcError>,
    ) -> Result<RpcOutput, RpcError> {
        if !self.config.process_list().contains(id) {
            return Err(RpcError::ProcessNotFound(id.to_string()));
        }

        let process = self
            .processes
            .get_mut(id)
            .expect("running process must in processes");
        operation(process)
    }
}
