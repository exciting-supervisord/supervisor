pub mod core;

use lib::config::Config;
use lib::request::Request;
use lib::response::Error as RpcError;

use std::io::Read;
use std::error::Error;
use std::os::unix::net::{UnixListener, UnixStream};
use std::thread;
use std::time::Duration;

use serde_json;

const CONF_FILE: &'static str = "./general.ini";

fn handle_client(mut socket: UnixStream) -> Result<Request, RpcError> {
    match serde_json::from_reader(socket) {
        Ok(r) => Ok(r),
        Err(e) => {
            
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let conf = Config::from(CONF_FILE)?;

    let listener = UnixListener::bind("/tmp/secret.sock")?;
    listener.set_nonblocking(true)?;

    loop {
        match listener.accept() {
            Ok((socket, addr)) => {
                match handle_client(socket) {
                    Ok(v) => println!("received: {:?}", v),
                    Err(e) => println!("err: {}", e)
                };
            }
            Err(e) => {
                println!("error occered: {}", e);
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}
