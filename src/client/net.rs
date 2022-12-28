extern crate jsonrpc;

use jsonrpc::simple_uds::{self, UdsTransport};
use jsonrpc::Client;

pub struct Net {
    sock_path: &'static str,
    client: Client,
}

impl Net {
    pub fn new(sock_path: &'static str) -> Self {
        let t = UdsTransport::new(sock_path); // ? 파일 없을 때?
        let client = Client::with_transport(t);
        Net { sock_path, client }
    }

    pub fn open(&mut self, sock_path: &str) {
        let t = UdsTransport::new(sock_path);
        self.client = Client::with_transport(t);
    }

    fn disconnect(&mut self) {}

    fn send_command(&mut self, words: Vec<&str>) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn recv_response(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }

    pub fn communicate_with_server(&mut self, words: Vec<&str>) {
        if let Err(e) = self.send_command(words) {
            eprintln!("Service Temporary Unavailable: {e:?}");
            self.disconnect();
        }
        if let Err(e) = self.recv_response() {
            eprintln!("Service Temporary Unavailable: {e:?}");
            self.disconnect();
        }
    }
}
