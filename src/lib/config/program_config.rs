use nix::sys::signal::Signal;
use std::collections::HashMap;
use std::error::Error;
use std::vec::Vec;

use super::autorestart::*;
use super::config_error::*;

#[derive(Debug, PartialEq)]
pub struct ProgramConfig {
    pub name: String,
    pub command: Vec<String>,
    pub numprocs: u32,
    pub stdout_logfile: String,
    pub stderr_logfile: String,
    pub directory: String,
    pub umask: Option<u32>,
    pub user: Option<String>,
    pub environment: HashMap<String, String>,

    pub autostart: bool,
    pub autorestart: AutoRestart,
    pub exitcodes: Vec<i32>,
    pub startsecs: u64,
    pub startretries: u32,
    pub stopsignal: Signal,
    pub stopwaitsecs: u64,
}

impl ProgramConfig {
    pub fn new(name: &str) -> Self {
        let exitcodes = vec![0];
        ProgramConfig {
            name: name.to_owned(),
            command: Vec::new(),
            numprocs: 1,
            autostart: false,
            autorestart: AutoRestart::Unexpected,
            exitcodes,
            startsecs: 1,
            startretries: 3,
            stopsignal: Signal::SIGTERM,
            stopwaitsecs: 10,
            stdout_logfile: String::from("/dev/null"),
            stderr_logfile: String::from("/dev/null"),
            directory: "/tmp".to_owned(),
            umask: None,
            user: None,
            environment: HashMap::new(),
        }
    }

    fn parse_exitcodes(k: &str, v: &str) -> Result<Vec<i32>, ConfigValueError> {
        let sp: Vec<String> = v.split(',').map(|x| x.to_owned()).collect();
        let mut vec = Vec::new();

        for s in sp {
            match s.parse() {
                Ok(o) => vec.push(o),
                Err(_) => return Err(ConfigValueError::new(k, v)),
            };
        }
        Ok(vec)
    }

    fn parse_environment(k: &str, v: &str) -> Result<HashMap<String, String>, ConfigValueError> {
        let sp: Vec<&str> = v.split(',').collect();
        let mut map = HashMap::new();

        for s in sp {
            let (key, value) = match s.split_once("=") {
                Some((key, value)) => (key.to_owned(), value.to_owned()),
                None => return Err(ConfigValueError::new(k, v)),
            };
            map.insert(key, value);
        }
        Ok(map)
    }

    fn parse_signal(k: &str, v: &str) -> Result<Signal, ConfigValueError> {
        match v {
            "KILL" => Ok(Signal::SIGKILL),
            "STOP" => Ok(Signal::SIGSTOP),
            "INT" => Ok(Signal::SIGINT),
            "QUIT" => Ok(Signal::SIGQUIT),
            "ILL" => Ok(Signal::SIGILL),
            "TRAP" => Ok(Signal::SIGTRAP),
            "ABRT" => Ok(Signal::SIGABRT),
            "BUS" => Ok(Signal::SIGBUS),
            "FPE" => Ok(Signal::SIGFPE),
            "USR1" => Ok(Signal::SIGUSR1),
            "SEGV" => Ok(Signal::SIGSEGV),
            "USR2" => Ok(Signal::SIGUSR2),
            "PIPE" => Ok(Signal::SIGPIPE),
            "ALRM" => Ok(Signal::SIGALRM),
            "TERM" => Ok(Signal::SIGTERM),
            "STKFLT" => Ok(Signal::SIGSTKFLT),
            "CHLD" => Ok(Signal::SIGCHLD),
            "CONT" => Ok(Signal::SIGCONT),
            "TSTP" => Ok(Signal::SIGTSTP),
            "TTIN" => Ok(Signal::SIGTTIN),
            "TTOU" => Ok(Signal::SIGTTOU),
            "URG" => Ok(Signal::SIGURG),
            "XCPU" => Ok(Signal::SIGXCPU),
            "XFSZ" => Ok(Signal::SIGXFSZ),
            "VTALRM" => Ok(Signal::SIGVTALRM),
            "PROF" => Ok(Signal::SIGPROF),
            "WINCH" => Ok(Signal::SIGWINCH),
            "IO" => Ok(Signal::SIGIO),
            "PWR" => Ok(Signal::SIGPWR),
            "SYS" => Ok(Signal::SIGSYS),
            _ => Err(ConfigValueError::new(k, v)),
        }
    }

    fn parse_umask(k: &str, v: &str) -> Result<u32, ConfigValueError> {
        let value_error = ConfigValueError::new(k, v);
        Ok(u32::from_str_radix(v, 8).map_err(|_| value_error)?)
    }

    fn parse_autorestart(k: &str, v: &str) -> Result<AutoRestart, ConfigValueError> {
        match v {
            "unexpected" => Ok(AutoRestart::Unexpected),
            "always" => Ok(AutoRestart::Always),
            "never" => Ok(AutoRestart::Never),
            _ => Err(ConfigValueError::new(k, v)),
        }
    }

    fn parse<T: std::str::FromStr>(k: &str, v: &str) -> Result<T, ConfigValueError> {
        let value_error = ConfigValueError::new(k, v);
        v.to_owned().parse::<T>().map_err(|_| value_error)
    }

    pub fn from(name: &str, prop: &ini::Properties) -> Result<Self, Box<dyn Error>> {
        let mut config = ProgramConfig::new(name);
        for (k, v) in prop.iter() {
            match k {
                "command" => config.command = v.split(' ').map(|x| x.to_owned()).collect(),
                "numprocs" => config.numprocs = ProgramConfig::parse::<u32>(k, v)?,
                "autostart" => config.autostart = ProgramConfig::parse::<bool>(k, v)?,
                "autorestart" => config.autorestart = ProgramConfig::parse_autorestart(k, v)?,
                "exitcodes" => config.exitcodes = ProgramConfig::parse_exitcodes(k, v)?,
                "startsecs" => config.startsecs = ProgramConfig::parse::<u64>(k, v)?,
                "startretries" => config.startretries = ProgramConfig::parse::<u32>(k, v)?,
                "stopsignal" => config.stopsignal = ProgramConfig::parse_signal(k, v)?,
                "stopwaitsecs" => config.stopwaitsecs = ProgramConfig::parse::<u64>(k, v)?,
                "stdout_logfile" => config.stdout_logfile = v.to_owned(),
                "stderr_logfile" => config.stderr_logfile = v.to_owned(),
                "directory" => config.directory = v.to_owned(),
                "umask" => config.umask = Some(ProgramConfig::parse_umask(k, v)? % 0o777),
                "user" => config.user = Some(v.to_owned()),
                "environment" => config.environment = ProgramConfig::parse_environment(k, v)?,
                _ => return Err(Box::new(ConfigKeyError::new(k))),
            }
        }
        if config.command.len() == 0 {
            return Err(Box::new(ConfigCommandError));
        }
        Ok(config)
    }

    pub fn diff(&self, other: &ProgramConfig) -> bool {
        self.stdout_logfile != other.stdout_logfile
            || self.stderr_logfile != other.stderr_logfile
            || self.directory != other.directory
            || self.umask != other.umask
            || self.user != other.user
            || self.environment != other.environment
            || self.autostart != other.autostart
            || self.autorestart != other.autorestart
            || self.exitcodes != other.exitcodes
            || self.startsecs != other.startsecs
            || self.startretries != other.startretries
            || self.stopsignal != other.stopsignal
            || self.stopwaitsecs != other.stopwaitsecs
            || self.command != other.command
    }
}

impl std::fmt::Display for ProgramConfig {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
