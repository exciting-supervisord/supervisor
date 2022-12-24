use std::io::prelude::*;
use std::io::ErrorKind;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

pub struct Net {
    sock_path: &'static str,
    stream: Option<UnixStream>,
}

impl Net {
    pub fn new(sock_path: &'static str) -> Self {
        Net {
            sock_path,
            stream: Net::connect(sock_path),
        }
    }

    fn connect(sock_path: &str) -> Option<UnixStream> {
        match UnixStream::connect(sock_path) {
            Ok(sock) => Some(sock),
            Err(_) => {
                eprintln!("{sock_path} refused connection");
                None
            }
        }
    }

    pub fn open(&mut self, sock_path: &str) {
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
        let mut stream = match self.stream {
            Some(ref sock) => sock,
            None => {
                return Err(std::io::Error::new(
                    ErrorKind::NotConnected,
                    format!("{} refused connection", self.sock_path),
                ))
            }
        };

        let mut line: String = words.join(" ");
        line.push('\0');
        stream.write_all(line.as_bytes())?;
        Ok(())
    }

    fn recv_response(&mut self) -> Result<(), std::io::Error> {
        let mut stream = match self.stream {
            Some(ref sock) => sock,
            None => {
                return Err(std::io::Error::new(
                    ErrorKind::NotConnected,
                    format!("{} refused connection", self.sock_path),
                ))
            }
        };

        let mut buf: [u8; 2] = [0; 2];
        let mut response = String::new();
        loop {
            let count = stream.read(&mut buf)?;
            if count == 0 {
                break;
            }
            response.push_str(&buf.map(|x| x as char).iter().collect::<String>());
        }
        println!("{response}");
        Ok(())
    }

    pub fn communicate_with_server(&mut self, words: Vec<&str>) {
        let net = self;

        if let Err(e) = net.send_command(words) {
            eprintln!("Service Temporary Unavailable: {e:?}");
            net.disconnect();
        }
        if let Err(e) = net.recv_response() {
            eprintln!("Service Temporary Unavailable: {e:?}");
            net.disconnect();
        }
    }
}
