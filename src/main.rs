mod client;
mod config;
mod server;

use client::client_main;
use server::server_main;
use std::thread;

fn main() {
    thread::spawn(|| client_main());
    server_main();
}
