use lib::logger::LOG;
use lib::request::{Procedure, ReqMethod, Request};
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
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct UdsRpcServer<ARG> {
    listener: UnixListener,
    methods: HashMap<String, Procedure<ARG>>,
    validator: Option<fn(&Request) -> Result<ARG, RpcError>>,
}

impl<ARG: 'static + Default> UdsRpcServer<ARG> {
    pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        let server = UdsRpcServer {
            listener: UnixListener::bind(path)?,
            methods: HashMap::new(),
            validator: None,
        };
        server.listener.set_nonblocking(true)?;
        set_permissions(path, Permissions::from_mode(0o600))?;

        Ok(server)
    }

    pub fn add_method<F>(&mut self, key: &str, method: F)
    where
        F: (Fn(ARG) -> Response) + 'static + Sync + Send,
    {
        self.methods.insert(key.to_string(), Arc::new(method));
    }

    pub fn set_validator(&mut self, validator: fn(&Request) -> Result<ARG, RpcError>) {
        self.validator = Some(validator)
    }

    fn get_request(&self, socket: &UnixStream) -> Result<ReqMethod<ARG>, RpcError> {
        let mut deserializer = serde_json::Deserializer::from_reader(socket);

        let req = Request::deserialize(&mut deserializer).map_err(|e| {
            LOG.warn(&format!("failed to receive request - {e}"));
            RpcError::service("request not received")
        })?; // FIXME 타임아웃..?

        let method = match self.methods.get(&req.method) {
            Some(m) => {
                LOG.info(&format!(
                    "new request received - method={}, argument={:?}",
                    &req.method, &req.args
                ));
                Ok(m.clone())
            }
            None => {
                LOG.warn(&format!("unknown method found - {}", &req.method));
                Err(RpcError::invalid_request("method"))
            }
        }?;

        let args = self.validate_request(&req)?;

        Ok(ReqMethod::new(method, args))
    }

    fn validate_request(&self, args: &Request) -> Result<ARG, RpcError> {
        if let Some(v) = self.validator.as_ref() {
            v(args)
        } else {
            Ok(ARG::default())
        }
    }

    fn handle_client(&self, socket: &UnixStream) {
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

        let res = req.run();
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

    pub fn accept_client(self: &Arc<Self>) {
        if let Ok((socket, ..)) = self.listener.accept() {
            let this = self.clone();

            thread::spawn(move || {
                this.handle_client(&socket);
            });
        } else {
            thread::sleep(Duration::from_millis(lib::EVENT_LOOP_TIME));
        }
    }
}

unsafe impl<A> Send for UdsRpcServer<A> {}

impl<A> Drop for UdsRpcServer<A> {
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
