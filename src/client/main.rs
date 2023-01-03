mod command;
mod net;
mod terminal;

extern crate lib;

use net::Net;
use std::error::Error;
use std::process;
use terminal::Terminal;

fn main() -> Result<(), Box<dyn Error>> {
    let sockfile = "/tmp/supervisor.sock";
    let mut t = Terminal::new("supervisor>");
    let mut net = Net::new(sockfile);

    loop {
        let line = t.getline()?;
        if line.is_empty() {
            continue;
        }
        match command::check_command(&line) {
            Err(_) => println!("*** Unknown syntax: {line}"),
            Ok(words) => match words[0] {
                "help" => command::print_help(words),
                "version" => command::print_version(),
                "open" => net.open(words[1]),
                "exit" | "quit" => process::exit(0),
                _ => net.communicate_with_server(words),
            },
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_client_main() {}
}
