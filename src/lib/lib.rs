pub mod config;
pub mod process_id;
pub mod process_status;
pub mod request;
pub mod response;

pub fn exit_with_error(err: Box<dyn std::error::Error>) -> ! {
    eprintln!("{err}");
    std::process::exit(1)
}
