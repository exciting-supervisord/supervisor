mod server;

use lib::config::Config;
use lib::request::Request;
use lib::response::{Error as RpcError, OutputMessage, Response};

use serde::Deserialize;
use server::supervisor::{self, Supervisor};

use core::cell::RefCell;
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

struct UdsRpcServer<'a> {
    listener: UnixListener,
    methods: HashMap<String, Box<dyn FnMut(Vec<String>) -> Response + 'a>>,
}

impl<'a> UdsRpcServer<'a> {
    pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        let server = UdsRpcServer {
            listener: UnixListener::bind(path)?,
            methods: HashMap::new(),
        };
        server.listener.set_nonblocking(true)?;
        Ok(server)
    }

    pub fn add_method<F>(&mut self, key: &str, method: F)
    where
        F: FnMut(Vec<String>) -> Response + 'a,
    {
        self.methods.insert(key.to_string(), Box::new(method));
    }

    fn get_method(&mut self, key: &str) -> &mut Box<dyn FnMut(Vec<String>) -> Response + 'a> {
        self.methods.get_mut(key).unwrap()
    }

    fn exec_method(&mut self, req: Request) -> Response {
        let method = self.get_method(&req.method);
        method(req.args)
    }

    fn get_request(&self, socket: &UnixStream) -> Result<Request, RpcError> {
        let mut deserializer = serde_json::Deserializer::from_reader(socket);
        let req = Request::deserialize(&mut deserializer)
            .map_err(|_| RpcError::service("request not received"))?;
        Ok(req)
    }

    fn handle_client(&mut self, socket: &UnixStream) {
        let req = match self.get_request(socket) {
            Ok(o) => o,
            Err(e) => {
                serde_json::to_writer(socket, &e);
                socket.shutdown(std::net::Shutdown::Both);
                return;
            }
        };

        let res = self.exec_method(req);
        serde_json::to_writer(socket, &res).or_else(|_| socket.shutdown(std::net::Shutdown::Both));
    }

    pub fn try_handle_client(&mut self) -> bool {
        if let Ok((ref socket, ..)) = self.listener.accept() {
            self.handle_client(socket);
            true
        } else {
            false
        }
    }
}

impl Drop for UdsRpcServer<'_> {
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

fn set_command_handlers<'a, 'b>(
    server: &'a mut UdsRpcServer<'b>,
    supervisor: &'b RefCell<Supervisor>,
) {
    let start = |args| supervisor.borrow_mut().start(args);
    let stop = |args| supervisor.borrow_mut().stop(args);
    // let update = |args| supervisor.update(args);

    server.add_method("start", start);
    server.add_method("stop", stop);
    // server.add_method("update", update);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    set_signal_handlers();

    let conf = match Config::from(CONF_FILE) {
        Ok(o) => o,
        Err(e) => lib::exit_with_error(e),
    };

    let supervisor = Supervisor::new(conf)?;
    let supervisor = RefCell::new(supervisor);
    let mut server = match UdsRpcServer::new(supervisor.borrow().sockfile()) {
        Ok(o) => o,
        Err(e) => lib::exit_with_error(e),
    };

    set_command_handlers(&mut server, &supervisor); // 'a 'b

    loop {
        server.try_handle_client();
        supervisor.borrow_mut().supervise()?;

        thread::sleep(Duration::from_millis(100));
        println!("loop");
        if SIGNALED.load(Ordering::Relaxed) {
            println!("Signal detected. cleaning up...");
            break;
        }
    }
    Ok(())
}
