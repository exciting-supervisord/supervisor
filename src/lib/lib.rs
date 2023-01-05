pub mod config;
pub mod logger;
pub mod process_id;
pub mod process_status;
pub mod request;
pub mod response;

pub const CONF_FILE: &'static str = "./general.ini";

pub fn exit_with_error(err: Box<dyn std::error::Error>) -> ! {
    eprintln!("{err}");
    std::process::exit(1)
}
