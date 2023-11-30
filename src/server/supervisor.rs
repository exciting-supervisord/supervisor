mod process;

use std::collections::HashMap;
use std::error::Error;
use std::mem::MaybeUninit;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard};
use std::vec::Vec;

use lib::config::{Config, ProgramConfig};
use lib::logger::LOG;
use lib::process_id::ProcessId;
use lib::process_status::ProcessStatus;
use lib::request::Request;
use lib::response::{
    Action, Error as RpcError, OutputMessage as RpcOutput, Response as RpcResponse,
};

use crate::net::UdsRpcServer;

use super::control;
use process::*;

pub type SupvArg = Vec<ProcessId>;

static mut SUPERVISOR: MaybeUninit<Mutex<Supervisor>> = MaybeUninit::uninit();

pub fn init(conf_file: &str, conf: Config) -> Result<(), Box<dyn Error>> {
    let supervisor = Supervisor::new(conf_file, conf)?;

    unsafe { SUPERVISOR.write(Mutex::new(supervisor)) };

    Ok(())
}

fn supervisor<'a>() -> MutexGuard<'a, Supervisor> {
    unsafe {
        SUPERVISOR
            .assume_init_ref()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }
}

pub fn supervise() -> Result<(), Box<dyn Error>> {
    supervisor().supervise()
}

pub fn update() {
    supervisor().update(Vec::new());
}

pub fn cleanup_processes() {
    supervisor().cleanup_processes()
}

pub fn register_rpc<'a>(server: &'a mut UdsRpcServer<SupvArg>) {
    let status = |args| supervisor().status(args);
    let start = |args| supervisor().start(args);
    let stop = |args| supervisor().stop(args);
    let shutdown = |args| supervisor().shutdown(args);
    let reload = |args| supervisor().reload(args);
    let update = |args| supervisor().update(args);
    let restart = |args| supervisor().restart(args);

    server.set_validator(|req| supervisor().validate(req));
    server.add_method("status", status);
    server.add_method("start", start);
    server.add_method("stop", stop);
    server.add_method("shutdown", shutdown);
    server.add_method("reload", reload);
    server.add_method("update", update);
    server.add_method("restart", restart);
}

pub struct Supervisor {
    file_path: String,
    config: Config,
    processes: HashMap<ProcessId, Process>,
    trashes: Vec<Process>,
}

impl Supervisor {
    fn new(file_path: &str, config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let mut sp = Supervisor {
            file_path: file_path.to_owned(),
            config: Default::default(),
            processes: HashMap::new(),
            trashes: Vec::new(),
        };

        for (_, v) in config.programs.iter() {
            for seq in 0..v.numprocs {
                sp.add_process(v, seq)?;
            }
        }
        sp.config = config;
        Ok(sp)
    }

    fn validate(&self, req: &Request) -> Result<SupvArg, RpcError> {
        if req.method == "status" && req.args.is_empty() {
            self.convert_to_process_ids(&vec![String::from("all")])
        } else {
            self.convert_to_process_ids(&req.args)
        }
    }

    fn supervise(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (_, process) in self.processes.iter_mut() {
            process.run()?;
        }
        self.garbage_collect();
        Ok(())
    }

    fn garbage_collect(&mut self) {
        self.trashes
            .iter_mut()
            .for_each(|p| p.run().unwrap_or_default());
        self.trashes.retain(|p| !p.is_stopped());
    }

    fn start(&mut self, inputs: SupvArg) -> RpcResponse {
        LOG.info(&format!("handle request - start, names={:?}", inputs));

        let act = inputs
            .iter()
            .map(|id| self.try_process_operation(id, Process::start))
            .collect::<Action>();
        RpcResponse::Action(act)
    }

    fn stop(&mut self, inputs: SupvArg) -> RpcResponse {
        LOG.info(&format!("handle request - stop, names={:?}", inputs));

        let act = inputs
            .iter()
            .map(|id| self.try_process_operation(id, Process::stop))
            .collect::<Action>();
        RpcResponse::Action(act)
    }

    fn restart(&mut self, inputs: SupvArg) -> RpcResponse {
        LOG.info(&format!("handle request - stop, names={:?}", inputs));

        let act = inputs
            .iter()
            .map(|process_id| {
                if let Some(mut process) = self.processes.remove(process_id) {
                    let ret = process.stop();
                    self.trashes.push(process);
                    ret
                } else {
                    Err(RpcError::ProcessNotFound(process_id.to_string()))
                }
            })
            .collect::<Action>();

        let act2 = inputs
            .iter()
            .map(|process_id| {
                if let Some(conf) = self.config.programs.get(&process_id.name) {
                    Process::new(conf, process_id.seq).and_then(|mut p| {
                        let ret = p.start();
                        self.processes.insert(p.get_id(), p);
                        ret
                    })
                } else {
                    Err(RpcError::ProcessNotFound(process_id.to_string()))
                }
            })
            .collect::<Action>();

        RpcResponse::Action(act + act2)
    }

    // Reload() -> ()
    fn reload(&mut self, _: SupvArg) -> RpcResponse {
        LOG.info("handle request - reload");
        self.cleanup_processes();

        let turn_on = self.config.process_list();
        for process_id in turn_on {
            self.revive_process(&process_id).unwrap_or_default();
        }
        RpcResponse::from_output(RpcOutput::new("taskmasterd", "reload"))
    }

    //     Shutdown() -> ()
    fn shutdown(&mut self, _: SupvArg) -> RpcResponse {
        LOG.info("handle request - shutdown");
        control::SHUTDOWN.store(true, Ordering::Relaxed);
        RpcResponse::from_output(RpcOutput::new("taskmasterd", "shutdown"))
    }

    fn remove_process(&mut self, process_id: &ProcessId) -> Result<(), RpcError> {
        if let Some(mut proc) = self.processes.remove(process_id) {
            proc.stop()?;
            self.trashes.push(proc);
        }
        Ok(())
    }

    fn add_process(&mut self, conf: &ProgramConfig, seq: u32) -> Result<(), RpcError> {
        let mut process = Process::new(conf, seq)?;
        if conf.autostart {
            process.start()?;
        }
        self.processes.insert(process.get_id(), process);

        Ok(())
    }

    fn revive_process(&mut self, process_id: &ProcessId) -> Result<(), RpcError> {
        let conf = self
            .config
            .programs
            .get(&process_id.name)
            .ok_or_else(|| RpcError::ProcessNotFound(process_id.to_string()))?;

        let mut process = Process::new(conf, process_id.seq)?;
        if conf.autostart {
            process.start()?;
        }
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
            let _ = self.remove_process(&process_id);
        }

        for process_id in turn_on {
            let program_conf = next_conf.programs.get(process_id.name.as_str()).unwrap();
            self.add_process(program_conf, process_id.seq)
                .unwrap_or_default();
        }
    }

    fn update(&mut self, _: SupvArg) -> RpcResponse {
        LOG.info("handle request - update");
        let next_conf = match Config::from(&self.file_path) {
            Ok(o) => o,
            Err(e) => return RpcResponse::from_err(RpcError::file_format(e.to_string().as_str())),
        };

        self.affect(&next_conf);
        self.config = next_conf;
        RpcResponse::from_output(RpcOutput::new("configuration", "updated"))
    }

    fn convert_to_process_ids(&self, names: &Vec<String>) -> Result<Vec<ProcessId>, RpcError> {
        if names.contains(&String::from("all")) {
            Ok(Vec::from_iter(self.config.process_list().into_iter()))
        } else {
            let mut v = Vec::new();
            for n in names.iter() {
                if let Some((name, seq)) = n
                    .split_once(":")
                    .and_then(|(name, seq)| seq.parse::<u32>().map(|seq| (name, seq)).ok())
                {
                    v.push(ProcessId::new(name.to_owned(), seq));
                } else {
                    return Err(RpcError::invalid_request("argument"));
                }
            }
            Ok(v)
        }
    }

    fn try_process_operation(
        &mut self,
        id: &ProcessId,
        operation: fn(id: &mut Process) -> Result<RpcOutput, RpcError>,
    ) -> Result<RpcOutput, RpcError> {
        if let Some(proc) = self.processes.get_mut(id) {
            operation(proc)
        } else {
            Err(RpcError::ProcessNotFound(id.to_string()))
        }
    }

    fn cleanup_processes(&mut self) {
        let keys: Vec<ProcessId> = self.processes.iter().map(|(k, _)| k.to_owned()).collect();

        for key in keys {
            let _ = self.remove_process(&key);
        }

        while self.trashes.len() != 0 {
            self.garbage_collect();
        }
    }

    // Status(Vec<name>) -> Result( Vec<ProcessStatus>, Error)
    // where Error: ServiceError + ProcessNotFoundError
    fn status(&self, words: SupvArg) -> RpcResponse {
        LOG.info("handle request - status");
        LOG.info(&format!("{:?}", words));

        let v: Vec<ProcessStatus> = words
            .iter()
            .map(|id| self.processes.get(id).unwrap().get_status())
            .collect();
        RpcResponse::Status(v)
    }
}
