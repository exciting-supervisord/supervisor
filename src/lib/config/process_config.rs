use nix::sys::signal::Signal;

use super::autorestart::*;
use super::program_config::*;

pub struct ProcessConfig {
    pub autostart: bool,
    pub autorestart: AutoRestart,
    pub exitcodes: Vec<i32>,
    pub startsecs: u64,
    pub startretries: u32,
    pub stopsignal: Signal,
    pub stopwaitsecs: u64,
}

impl ProcessConfig {
    pub fn from(conf: &ProgramConfig) -> Self {
        ProcessConfig {
            autostart: conf.autostart,
            autorestart: conf.autorestart,
            exitcodes: conf.exitcodes.clone(),
            startsecs: conf.startsecs,
            startretries: conf.startretries,
            stopsignal: conf.stopsignal,
            stopwaitsecs: conf.stopwaitsecs,
        }
    }
}
