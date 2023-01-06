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
    let conf_file = if args.len() > 1 { &args[1] } else { CONF_FILE };
    println!("\nConfiguration file: {conf_file}\n");
    println!("If you want to set the other configuration file, put it on the first argument.\n");
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
