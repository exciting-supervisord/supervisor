mod command_messages;

use lib::TM_VERSION;

fn check_argument_syntax<'a>(words: Vec<&'a str>, help: Vec<&'a str>) -> Vec<&'a str> {
    for w in &words[1..] {
        let w = *w;
        if w == "all" {
            continue;
        }
        match w.split_once(":") {
            None => return help,
            Some((_, seq)) => {
                if let Err(_) = seq.parse::<u32>() {
                    return help;
                }
            }
        }
    }
    words
}

fn check_arguments(error_message: String, words: Vec<&str>, count: usize) -> Vec<&str> {
    let help = vec!["help", words[0]];
    if words.len() != count + 1 {
        println!("{}", error_message);
        return help;
    }
    if words[0] == "open" {
        return words;
    }
    check_argument_syntax(words, help)
}

pub fn check_command(line: &str) -> Result<Vec<&str>, ()> {
    let words: Vec<&str> = line.split(' ').filter(|x| !x.is_empty()).collect();

    match words[0] {
        "version" | "reload" => Ok(check_arguments(
            format!("Error: {} accepts no arguments", words[0].to_owned()),
            words,
            0,
        )),
        "open" => Ok(check_arguments(
            format!("ERROR: path must be /path/to/socket"),
            words,
            1,
        )),
        "start" | "stop" | "restart" => Ok(check_arguments(
            format!("Error: {} requires a process name", words[0].to_owned()),
            words,
            1,
        )),
        "status" | "shutdown" | "update" | "quit" | "exit" | "help" => Ok(words),
        _ => Err(()),
    }
}

pub fn print_help(words: Vec<&str>) {
    if words.len() == 1 {
        println!("{}", command_messages::HELP);
        return;
    }
    match words[1] {
        "restart" => println!("{}", command_messages::HELP_RESTART),
        "start" => println!("{}", command_messages::HELP_START),
        "stop" => println!("{}", command_messages::HELP_STOP),
        "status" => println!("{}", command_messages::HELP_STATUS),
        "open" => println!("{}", command_messages::HELP_OPEN),
        "reload" => println!("{}", command_messages::HELP_RELOAD),
        "shutdown" => println!("{}", command_messages::HELP_SHUTDOWN),
        "update" => println!("{}", command_messages::HELP_UPDATE),
        "quit" => println!("{}", command_messages::HELP_QUIT),
        "exit" => println!("{}", command_messages::HELP_EXIT),
        "version" => println!("{}", command_messages::HELP_VERSION),
        "help" => println!("{}", command_messages::HELP_HELP),
        _ => {
            let s = words[1..].join(" ");
            eprintln!("*** No help on {}", s);
        }
    }
}

pub fn print_version() {
    println!("{TM_VERSION}");
}
