mod command;
mod net;
mod terminal;

extern crate lib;

use lib::config::Config;
use lib::CONF_FILE;

use std::env;
use std::error::Error;
use std::process;

use net::Net;
use terminal::Terminal;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let conf_file = match args.len() {
        1 => CONF_FILE,
        2 => &args[1],
        _ => {
            eprintln!("usage: {} [conf_file]", &args[0]);
            eprintln!("if conf_file is missing, default ({CONF_FILE}) will be used.");
            std::process::exit(1);
        }
    };

    let conf = Config::from(conf_file).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    let mut terminal = Terminal::new("taskmaster>");
    let mut net = Net::new(&conf.general.sockfile);

    loop {
        let line = terminal.getline()?;
        if line.is_empty() {
            continue;
        }
        match command::check_command(&line) {
            Err(_) => println!("*** Unknown syntax: {line}"),
            Ok(words) => match words[0] {
                "help" => command::print_help(words),
                "version" => command::print_version(),
                "open" => {
                    net.open(words[1]);
                    net.communicate_with_server(vec!["status"]);
                }
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
