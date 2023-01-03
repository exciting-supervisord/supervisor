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
        Net {
            sock_path: sock_path.to_owned(),
            stream: Net::connect(sock_path),
        }
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
            format!("{} refused connection", self.sock_path),
        ))?;

        let line: String = serde_json::to_string::<Request>(&req)?;
        println!("{line}");
        stream.write_all(line.as_bytes())?;
        Ok(())
    }

    fn recv_response(&mut self) -> Result<(), std::io::Error> {
        let mut stream = self.stream.as_ref().ok_or(std::io::Error::new(
            ErrorKind::NotConnected,
            format!("{} refused connection", self.sock_path),
        ))?;

        let mut line = String::new();
        stream.read_to_string(&mut line)?;
        let responses = serde_json::from_str::<Response>(&line)?;

        responses.list.iter().for_each(|res| match res {
            Ok(o) => println!("{o}"),
            Err(e) => eprintln!("{e}"),
        });
        Ok(())
    }

    pub fn communicate_with_server(&mut self, words: Vec<&str>) {
        self.stream = Net::connect(self.sock_path.as_str());
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
