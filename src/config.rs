mod config {
    use std::collections::HashMap;
    use std::vec::Vec;

    pub enum AutoRestart {
        Unexpected,
        Always,
        Never,
    }

    pub struct ProgramConfig {
        command: Vec<String>,
        numprocs: u32,
        autostart: bool,
        autorestart: AutoRestart,
        exitcodes: Vec<u8>,
        startsecs: u32,
        startretries: u32,
        stopsignal: u8, // TODO: use type like enum(SIGINT, ...)
        stopwaitsecs: u32,
        stdout_logfile: String,
        stderr_logfile: String,
        directory: String,
        umask: Option<u8>,
        user: Option<String>,
        enviroment: HashMap<String, String>,
    }

    impl ProgramConfig {
        // pub fn from(string: str)
    }
}
