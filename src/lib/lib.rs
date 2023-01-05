pub mod config;
pub mod logger;
pub mod process_id;
pub mod process_status;
pub mod request;
pub mod response;
pub mod daemon;

pub const CONF_FILE: &'static str = "./general.ini";
pub const LOG_FILE: &'static str = "/tmp/taskmaster.log";
pub const TM_VERSION: &'static str = "0.0.1";

pub fn exit_with_error(err: Box<dyn std::error::Error>) -> ! {
    logger::LOG.crit(&format!("{err}"));
    std::process::exit(1)
}
