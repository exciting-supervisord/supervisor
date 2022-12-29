mod core;

use std::{
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
};

fn handle_client(mut stream: UnixStream) {
    let mut buffer: [u8; 10] = [0; 10];

    match stream.read(&mut buffer) {
        Ok(_) => println!("server: {buffer:?}"),
        Err(e) => {
            println!("server: Couldn't read: {e:?}");
            return;
        }
    };

    match stream.write_all(&mut buffer) {
        Ok(_) => {}
        Err(e) => {
            println!("server: Couldn't write: {e:?}");
            return;
        }
    };

    match stream.write_all(&mut buffer) {
        Ok(_) => {}
        Err(e) => {
            println!("server: Couldn't write: {e:?}");
            return;
        }
    };
}

fn main() {
    println!("server: started");
    let listener = match UnixListener::bind("/tmp/supervisor.sock") {
        Ok(l) => l,
        Err(e) => {
            println!("{e:?}");
            return;
        }
    };

    // accept connections and process them, spawning a new thread for each one
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                /* connection succeeded */
                handle_client(stream);
            }
            Err(_) => {
                /* connection failed */
                break;
            }
        }
    }

    println!("server: died");
}
