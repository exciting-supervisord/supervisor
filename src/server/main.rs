mod server;

use lib::config::Config;
use lib::request::Request;
use lib::response::{Error as RpcError, OutputMessage, Response};

use serde::Deserialize;
use server::supervisor::Supervisor;

use std::collections::HashMap;
use std::error::Error;
use std::fs::remove_file;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde_json;

use nix::sys::signal::{self, SigHandler, Signal};

const CONF_FILE: &'static str = "./general.ini";

struct UdsRpcServer {
    listener: UnixListener,
    methods: HashMap<String, fn(&Vec<String>) -> Response>,
}

impl UdsRpcServer {
    pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        let server = UdsRpcServer {
            listener: UnixListener::bind(path)?,
            methods: HashMap::new(),
        };
        // server.listener.set_nonblocking(true)?;

        Ok(server)
    }

    pub fn add_method(&mut self, key: &str, method: fn(&Vec<String>) -> Response) {
        self.methods.insert(key.to_string(), method);
    }

    fn get_method(&self, key: &str) -> Result<&fn(&Vec<String>) -> Response, RpcError> {
        match self.methods.get(key) {
            Some(method) => Ok(method),
            None => Err(RpcError::Service),
        }
    }

    fn exec_method(&self, socket: &UnixStream) -> Result<OutputMessage, RpcError> {
        // let req: Request = serde_json::from_reader(socket).map_err(|_| RpcError::Service)?;
        
        let mut de = serde_json::Deserializer::from_reader(socket);

        let req = Request::deserialize(&mut de).map_err(|_| RpcError::Service)?;
        
        let method = self.get_method(&req.method)?;
        method(&req.args)
    }

    pub fn try_handle_client(&self) -> Result<bool, RpcError> {
        match self.listener.accept() {
            Ok((ref socket, ..)) => {
                match self.exec_method(socket) {
                    Ok(msg) => {
                        let to_send: Response = Ok(msg);
                        serde_json::to_writer(socket, &to_send).map_err(|_| RpcError::Service)?;
                        Ok(true)
                    }
                    Err(e) => Err(e),
                }?;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }
}

impl Drop for UdsRpcServer {
    fn drop(&mut self) {
        remove_file(self.listener.local_addr().unwrap().as_pathname().unwrap()).unwrap();
    }
}

static SIGNALED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigint(signal: libc::c_int) {
    let signal = Signal::try_from(signal).unwrap();
    SIGNALED.store(
        Signal::SIGINT == signal || Signal::SIGTERM == signal,
        Ordering::Relaxed,
    );
}

fn set_signal_handlers() {
    let handler = SigHandler::Handler(handle_sigint);
    unsafe {
        signal::signal(Signal::SIGINT, handler).expect("signal");
        signal::signal(Signal::SIGTERM, handler).expect("signal");
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    set_signal_handlers();
    let conf = Config::from(CONF_FILE)?;
    let mut server = UdsRpcServer::new(&conf.general.sockfile)?;

    server.add_method("start", |v| {
        println!("vec = {:?}", v);
        Ok(OutputMessage::new("test", "good"))
    });

    let mut supervisor = Supervisor::new(conf)?;
    loop {
        server.try_handle_client()?;
        supervisor.supervise()?;

        thread::sleep(Duration::from_millis(100));
        println!("loop");
        if SIGNALED.load(Ordering::Relaxed) {
            println!("Signal detected. cleaning up...");
            break;
        }
    }
    Ok(())
}
