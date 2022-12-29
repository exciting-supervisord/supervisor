mod command_messages;

pub fn check_arguments(error_message: String, words: Vec<&str>, count: usize) -> Vec<&str> {
    if words.len() != count + 1 {
        println!("{}", error_message);
        vec!["help", words[0]]
    } else {
        words
    }
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
        "add" | "remove" | "avail" | "status" | "shutdown" | "update" | "log" | "quit" | "exit"
        | "help" => Ok(words),
        _ => Err(()),
    }
}

pub fn print_help(words: Vec<&str>) {
    if words.len() == 1 {
        println!("{}", command_messages::HELP);
        return;
    }
    match words[1] {
        // "add" => println!("{}", command_messages::HELP_ADD),
        // "remove" => println!("{}", command_messages::HELP_REMOVE),
        // "avail" => println!("{}", command_messages::HELP_AVAIL),
        "restart" => println!("{}", command_messages::HELP_RESTART),
        "start" => println!("{}", command_messages::HELP_START),
        "stop" => println!("{}", command_messages::HELP_STOP),
        "status" => println!("{}", command_messages::HELP_STATUS),
        "open" => println!("{}", command_messages::HELP_OPEN),
        "reload" => println!("{}", command_messages::HELP_RELOAD),
        "shutdown" => println!("{}", command_messages::HELP_SHUTDOWN),
        "update" => println!("{}", command_messages::HELP_UPDATE),
        "log" => println!("{}", command_messages::HELP_QUIT), // FIXME
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
    println!("Which version must be printed?"); // FIXME
}
