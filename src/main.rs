mod client;
mod server;

use client::client_main;
use server::server_main;
use std::{thread, time};

fn main() {
    thread::spawn(|| client_main());
    server_main();
}
