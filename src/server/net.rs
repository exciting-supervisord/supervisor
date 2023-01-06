use lib::logger::LOG;
use lib::request::Request;
use lib::response::{Error as RpcError, Response};

use serde::Deserialize;
use serde_json;

use std::collections::HashMap;
use std::error::Error;
use std::fs::remove_file;
use std::fs::set_permissions;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
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
        set_permissions(path, Permissions::from_mode(0o600))?;

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
        let req = Request::deserialize(&mut deserializer).map_err(|e| {
            LOG.warn(&format!("failed to receive request - {e}"));
            RpcError::service("request not received")
        })?; // FIXME 타임아웃..?

        if let None = self.methods.get(&req.method) {
            LOG.warn(&format!("unknown method found - {}", &req.method));
            return Err(RpcError::invalid_request("method"));
        }

        if req.method != "open" {
            if let Err(e) = self.args_validation(&req.args) {
                LOG.warn(&format!(
                    "invalid argument for given method - method={}, argument={:?}",
                    &req.method, &req.args
                ));
                return Err(e);
            }
        }

        LOG.info(&format!(
            "new request received - method={}, argument={:?}",
            &req.method, &req.args
        ));
        Ok(req)
    }

    fn args_validation(&self, args: &Vec<String>) -> Result<(), RpcError> {
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
                LOG.warn(&format!("failed to handle client - {e}"));
                serde_json::to_writer(socket, &Response::from_err(e)).unwrap_or_default();
                socket
                    .shutdown(std::net::Shutdown::Both)
                    .unwrap_or_default();
                return;
            }
        };

        let res = self.exec_method(req);
        if let Err(e) = serde_json::to_writer(socket, &res) {
            LOG.warn(&format!(
                "fail to resoponse to client - response={}, error={e}",
                res
            ));
            socket
                .shutdown(std::net::Shutdown::Both)
                .unwrap_or_default();
        }

        LOG.info(&format!("request handled - response=\n{}", res));
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
        let socket_file = self.listener.local_addr().unwrap();
        let socket_file = socket_file.as_pathname().unwrap();
        LOG.info(&format!(
            "remove socket file - {}",
            socket_file.to_str().unwrap()
        ));
        remove_file(socket_file).unwrap()
    }
}
