use std::io::prelude::*;
use std::io::ErrorKind;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

use lib::request::Request;
use lib::response::Response;

pub struct Net {
    sock_path: String,
    stream: Option<UnixStream>,
}

impl Net {
    pub fn new(sock_path: &str) -> Self {
        let mut net = Net {
            sock_path: sock_path.to_owned(),
            stream: None,
        };
        net.communicate_with_server(vec!["status"]);
        net
    }

    fn connect(sock_path: &str) -> Option<UnixStream> {
        let ret = UnixStream::connect(sock_path).ok();
        if let None = ret {
            eprintln!("{sock_path} refused connection");
        }
        ret
    }

    pub fn open(&mut self, sock_path: &str) {
        self.sock_path = sock_path.to_owned();
        self.stream = Net::connect(sock_path); // 이전 소켓 있을 때?
    }

    fn disconnect(&mut self) {
        let stream = match self.stream {
            Some(ref sock) => sock,
            None => return,
        };

        stream
            .shutdown(Shutdown::Both)
            .expect("connection shutdown failed");
    }

    fn send_command(&mut self, words: Vec<&str>) -> Result<(), std::io::Error> {
        let req = Request::from(&words);
        let mut stream = self.stream.as_ref().ok_or(std::io::Error::new(
            ErrorKind::NotConnected,
            format!("not connected"),
        ))?;

        let line: String = serde_json::to_string::<Request>(&req)?;
        stream.write_all(line.as_bytes())?;
        Ok(())
    }

    fn recv_response(&mut self) -> Result<(), std::io::Error> {
        let mut stream = self.stream.as_ref().ok_or(std::io::Error::new(
            ErrorKind::NotConnected,
            format!("not connected"),
        ))?;

        let mut line = String::new();
        stream.read_to_string(&mut line)?;
        let responses = serde_json::from_str::<Response>(&line)?;

        match responses {
            Response::Action(act) => act.list.iter().for_each(|res| match res {
                Ok(o) => println!("{o}"),
                Err(e) => eprintln!("{e}"),
            }),
            Response::Status(stat) => stat.iter().for_each(|x| println!("{x}")),
        }
        Ok(())
    }

    pub fn communicate_with_server(&mut self, words: Vec<&str>) {
        self.stream = Net::connect(self.sock_path.as_str());
        if let Err(e) = self.send_command(words) {
            eprintln!("Service temporary unavailable: {e}");
            self.disconnect();
            return;
        }
        if let Err(e) = self.recv_response() {
            eprintln!("Service temporary unavailable: {e}");
            self.disconnect();
        }
    }
}
