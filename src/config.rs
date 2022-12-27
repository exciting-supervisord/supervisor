extern crate nix;

mod config_error;
mod parser_ini;

mod config {
    use super::{config_error::*, parser_ini};
    use nix::sys::signal::Signal;
    use std::collections::HashMap;
    use std::error::Error;
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
        stopsignal: Signal,
        stopwaitsecs: u32,
        stdout_logfile: String,
        stderr_logfile: String,
        directory: String,
        umask: Option<u8>,
        user: Option<String>,
        enviroment: HashMap<String, String>,
    }

    impl ProgramConfig {
        fn new() -> Self {
            let exitcodes = vec![0];
            ProgramConfig {
                command: Default::default(),
                numprocs: 1,
                autostart: false,
                autorestart: AutoRestart::Unexpected,
                exitcodes,
                startsecs: 1,
                startretries: 3,
                stopsignal: Signal::SIGTERM,
                stopwaitsecs: 10,
                stdout_logfile: String::new(),
                stderr_logfile: String::new(),
                directory: "/tmp".to_owned(),
                umask: None,
                user: None,
                enviroment: Default::default(),
            }
        }

        fn parse_exitcodes(k: &str, v: &str) -> Result<Vec<u8>, ConfigValueError> {
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

        fn parse_environment(
            k: &str,
            v: &str,
        ) -> Result<HashMap<String, String>, ConfigValueError> {
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
            let valueError = ConfigValueError::new(k, v);
            match v {
                // TODO 시그널 전부...?
                "INT" => Ok(Signal::SIGINT),
                "QUIT" => Ok(Signal::SIGQUIT),
                "TERM" => Ok(Signal::SIGTERM),
                "KILL" => Ok(Signal::SIGKILL),
                "STOP" => Ok(Signal::SIGSTOP),
                _ => Err(valueError),
            }
        }

        fn parse<T: std::str::FromStr>(k: &str, v: &str) -> Result<T, ConfigValueError> {
            let valueError = ConfigValueError::new(k, v);
            v.to_owned().parse::<T>().map_err(|_| valueError)
        }

        pub fn from(prop: &ini::Properties) -> Result<Self, Box<dyn Error>> {
            let mut config = ProgramConfig::new();
            for (k, v) in prop.iter() {
                match k {
                    "command" => config.command = v.split(' ').map(|x| x.to_owned()).collect(),
                    "numprocs" => config.numprocs = ProgramConfig::parse::<u32>(k, v)?,
                    "autostart" => config.autostart = ProgramConfig::parse::<bool>(k, v)?,
                    "exitcodes" => config.exitcodes = ProgramConfig::parse_exitcodes(k, v)?,
                    "startsecs" => config.startsecs = ProgramConfig::parse::<u32>(k, v)?,
                    "startretries" => config.startretries = ProgramConfig::parse::<u32>(k, v)?,
                    "stopsignal" => config.stopsignal = ProgramConfig::parse_signal(k, v)?,
                    "stopwaitsecs" => config.stopwaitsecs = ProgramConfig::parse::<u32>(k, v)?,
                    "stdout_logfile" => config.stdout_logfile = v.to_owned(),
                    "stderr_logfile" => config.stderr_logfile = v.to_owned(),
                    "directory" => config.directory = v.to_owned(),
                    "umask" => config.umask = Some(u8::from_str_radix(v, 8)?),
                    "user" => config.user = Some(v.to_owned()),
                    "enviroment" => config.enviroment = ProgramConfig::parse_environment(k, v)?,
                    _ => return Err(Box::new(ConfigKeyError::new(k))),
                }
            }
            if config.command.len() == 0 {
                return Err(Box::new(ConfigCommandError));
            }
            Ok(config)
        }
    }

    pub struct GeneralConfig {
        sockfile: String,
        pidfile: String,
    }

    impl GeneralConfig {
        pub fn new() -> Self {
            GeneralConfig {
                sockfile: "/tmp/supervisord.sock".to_owned(),
                pidfile: "/tmp/supervisord.pid".to_owned(),
            }
        }

        pub fn from(prop: &ini::Properties) -> Result<Self, Box<dyn Error>> {
            let mut config = GeneralConfig::new();
            for (k, v) in prop.iter() {
                match k {
                    "sockfile" => config.sockfile = v.to_owned(),
                    "pidfile" => config.pidfile = v.to_owned(),
                    _ => return Err(Box::new(ConfigKeyError::new(k))),
                }
            }
            Ok(config)
        }
    }

    struct Config {
        general: GeneralConfig,
        programs: HashMap<String, ProgramConfig>,
    }

    impl Config {
        fn from(file_path: &str) -> Result<Self, Box<dyn Error>> {
            let ini = parser_ini::load_ini(file_path)?;
            let mut general = GeneralConfig::new();
            let mut programs = HashMap::new();
            for (sec, prop) in ini.iter() {
                match sec {
                    None => {}
                    Some("general") => general = GeneralConfig::from(prop)?,
                    Some(sec) => {
                        if let Some((key, value)) = sec.split_once(":") {
                            if let "program" = key {
                                programs.insert(value.to_owned(), ProgramConfig::from(prop)?);
                            }
                        }
                    }
                }
            }
            Ok(Config { general, programs })
        }
    }
}

// pub struct Properties {
//     data: ListOrderedMultimap<PropertyKey, String>,
// }
