mod command_messages;
mod net;
mod terminal;

use net::Net;
use std::process;
use std::{thread, time};
use terminal::Terminal;

fn check_arguments(error_message: String, words: Vec<&str>) -> Vec<&str> {
    if words.len() == 1 {
        println!("{}", error_message);
        vec!["help", words[0]]
    } else {
        words
    }
}

fn check_command(line: &str) -> Result<Vec<&str>, ()> {
    let words: Vec<&str> = line.split(' ').filter(|x| !x.is_empty()).collect();

    match words[0] {
        "start" | "stop" | "restart" => Ok(check_arguments(
            format!("Error: {} requires a process name", words[0].to_owned()),
            words,
        )),
        "version" => Ok(check_arguments(
            format!("Error: version accepts no arguments"),
            words,
        )),
        "add" | "remove" | "avail" | "status" | "open" | "reload" | "shutdown" | "update"
        | "log" | "quit" | "exit" | "help" => Ok(words),
        _ => Err(()),
    }
}

fn print_help(words: Vec<&str>) {
    if words.len() == 1 {
        println!("{}", command_messages::HELP);
        return;
    }
    match words[1] {
        "add" => println!("{}", command_messages::HELP_ADD),
        "remove" => println!("{}", command_messages::HELP_REMOVE),
        "avail" => println!("{}", command_messages::HELP_AVAIL),
        "restart" => println!("{}", command_messages::HELP_RESTART),
        "start" => println!("{}", command_messages::HELP_START),
        "stop" => println!("{}", command_messages::HELP_STOP),
        "status" => println!("{}", command_messages::HELP_STATUS),
        // "open" => println!("{}", help_messages::HELP_AVAIL),
        // "reload" => println!("{}", help_messages::HELP_AVAIL),
        "shutdown" => println!("{}", command_messages::HELP_SHUTDOWN),
        "update" => println!("{}", command_messages::HELP_UPDATE),
        "log" => println!("{}", command_messages::HELP_AVAIL), // FIXME
        "quit" => println!("{}", command_messages::HELP_QUIT),
        "exit" => println!("{}", command_messages::HELP_EXIT),
        "version" => println!("{}", command_messages::HELP_VERSION),
        "help" => println!("{}", command_messages::HELP_HELP),
        _ => {
            let s = words[1..].join(" ");
            println!("*** No help on {}", s);
        }
    }
}

fn print_version() {
    println!("Which version must be printed?"); // FIXME
}

pub fn client_main() {
    thread::sleep(time::Duration::from_millis(1000));
    let mut t = Terminal::new("supervisor>");
    let mut net = Net::new("/tmp/supervisor.sock");

    loop {
        let line = t.getline();
        if line.is_empty() {
            continue;
        }
        match check_command(&line) {
            Err(_) => println!("*** Unknown syntax: {line}"),
            Ok(words) => match words[0] {
                "help" => print_help(words),
                "version" => print_version(),
                "exit" | "quit" => process::exit(0),
                _ => net.communicate_with_server(words),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_main() {}
}
