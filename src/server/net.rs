use lib::request::Request;
use lib::response::{Error as RpcError, Response};

use serde::Deserialize;

use std::collections::HashMap;
use std::error::Error;
use std::fs::remove_file;
use std::os::unix::net::{UnixListener, UnixStream};

use serde_json;

pub struct UdsRpcServer<'a> {
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
            .map_err(|_| RpcError::service("request not received"))?; // FIXME 타임아웃..?

        if let None = self.methods.get(&req.method) {
            return Err(RpcError::invalid_request("method"));
        }

        if let Err(e) = self.args_validation(&req.args) {
            return Err(e);
        }
        Ok(req)
    }

    fn args_validation(&self, args: &Vec<String>) -> Result<(), RpcError> {
        println!("{:?}", args);
        for a in args {
            if a == "all" {
                continue;
            }

            match a.split_once(":") {
                None => return Err(RpcError::invalid_request("argument")),
                Some((_, seq)) => {
                    if let Err(_) = seq.parse::<u32>() {
                        return Err(RpcError::invalid_request(a));
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_client(&mut self, socket: &UnixStream) {
        let req = match self.get_request(socket) {
            Ok(o) => o,
            Err(e) => {
                serde_json::to_writer(socket, &Response::from_err(e));
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
