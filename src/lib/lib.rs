pub mod config;
pub mod daemon;
pub mod logger;
pub mod process_id;
pub mod process_status;
pub mod request;
pub mod response;

pub const CONF_FILE: &'static str = "/etc/tmd/taskmaster.ini";
pub const LOG_FILE: &'static str = "/tmp/taskmaster.log";
pub const TM_VERSION: &'static str = "0.0.1";
pub const EVENT_LOOP_TIME: u64 = 50;

pub fn exit_with_log(err: Box<dyn std::error::Error>) -> ! {
    logger::LOG.crit(&format!("{err}"));
    std::process::exit(1)
}
