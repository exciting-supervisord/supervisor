extern crate jsonrpc;

use jsonrpc::simple_uds::UdsTransport;
use jsonrpc::Client;
use serde_json::value::RawValue;
use std::error::Error;

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

    fn arguments_to_params(&self, words: &Vec<&str>) -> Result<Vec<Box<RawValue>>, Box<dyn Error>> {
        let mut v: Vec<Box<RawValue>> = Default::default();
        if words.len() == 1 {
            return Ok(v);
        }

        let mut s = String::new();
        s.push_str("[ ");
        s.push_str("\"");
        s.push_str(words[0]);
        s.push_str("\"");
        for w in words[1..].iter() {
            s.push_str(", \"");
            s.push_str(w);
            s.push_str("\"");
        }
        s.push_str(" ]");
        v.push(RawValue::from_string(s)?);
        Ok(v)
    }

    pub fn health_check(&self) -> Result<(), Box<dyn Error>> {
        let request = self.client.build_request("health_check", &[]);
        let response = self.client.send_request(request);

        match response {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("{} refused connection", self.sock_path);
                Err(Box::new(e))
            }
        }
    }

    pub fn communicate_with_server(&mut self, words: Vec<&str>) -> Result<(), Box<dyn Error>> {
        let params = self.arguments_to_params(&words)?;
        let request = self.client.build_request(words[0], &params);
        // 소켓파일 없을 때 여기서 에러 날 듯..?
        let respone = match self.client.send_request(request) {
            Ok(o) => o,
            Err(e) => panic!("68: {e}"),
        };
        match respone.result::<String>() {
            Ok(o) => println!("{o}"),
            Err(e) => eprintln!("{}", e),
        };
        Ok(())
    }
}
