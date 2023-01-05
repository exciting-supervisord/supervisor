extern crate nix;

mod config_error;
mod parser_ini;

use super::process_id::ProcessId;
use config_error::*;
use nix::sys::signal::Signal;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::vec::Vec;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AutoRestart {
    Unexpected,
    Always,
    Never,
}

pub struct ProcessConfig {
    pub autostart: bool,
    pub autorestart: AutoRestart,
    pub exitcodes: Vec<i32>,
    pub startsecs: u64,
    pub startretries: u32,
    pub stopsignal: Signal,
    pub stopwaitsecs: u64,
}

impl ProcessConfig {
    pub fn from(conf: &ProgramConfig) -> Self {
        ProcessConfig {
            autostart: conf.autostart,
            autorestart: conf.autorestart,
            exitcodes: conf.exitcodes.clone(),
            startsecs: conf.startsecs,
            startretries: conf.startretries,
            stopsignal: conf.stopsignal,
            stopwaitsecs: conf.stopwaitsecs,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ProgramConfig {
    pub name: String,
    pub command: Vec<String>,
    pub numprocs: u32, // ?
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
    fn new(name: &str) -> Self {
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
            // TODO 시그널 전부...?
            "INT" => Ok(Signal::SIGINT),
            "QUIT" => Ok(Signal::SIGQUIT),
            "TERM" => Ok(Signal::SIGTERM),
            "KILL" => Ok(Signal::SIGKILL),
            "STOP" => Ok(Signal::SIGSTOP),
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
            || self.command != self.command
    }
}

impl std::fmt::Display for ProgramConfig {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct GeneralConfig {
    pub sockfile: String,
    pub pidfile: String,
}

impl GeneralConfig {
    pub fn new() -> Self {
        GeneralConfig {
            sockfile: "/tmp/supervisord.sock".to_owned(),
            pidfile: "/tmp/supervisord.pid".to_owned(),
        }
    }

    pub fn from(prop: &ini::Properties) -> Result<Self, ConfigKeyError> {
        let mut config = GeneralConfig::new();
        for (k, v) in prop.iter() {
            match k {
                "sockfile" => config.sockfile = v.to_owned(),
                "pidfile" => config.pidfile = v.to_owned(),
                _ => return Err(ConfigKeyError::new(k)),
            }
        }
        Ok(config)
    }
}

#[derive(Debug, PartialEq)]
pub struct Config {
    pub general: GeneralConfig,
    pub programs: HashMap<String, ProgramConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            general: GeneralConfig::new(),
            programs: Default::default(),
        }
    }
}

impl Config {
    pub fn from(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let ini = parser_ini::load_ini(file_path).map_err(|_| ConfigFileError)?;
        let mut general = GeneralConfig::new();
        let mut programs = HashMap::new();
        for (sec, prop) in ini.iter() {
            match sec {
                None => {}
                Some("general") => general = GeneralConfig::from(prop)?,
                Some(sec) => {
                    if let Some((key, value)) = sec.split_once(":") {
                        if let "program" = key {
                            programs.insert(value.to_owned(), ProgramConfig::from(value, prop)?);
                        }
                    }
                }
            }
        }
        Ok(Config { general, programs })
    }

    pub fn process_list(&self) -> HashSet<ProcessId> {
        let mut set = HashSet::new();
        self.programs.iter().for_each(|(k, v)| {
            for process_num in 0..v.numprocs {
                match set.insert(ProcessId::new(k.to_owned(), process_num)) {
                    _ => {}
                }
            }
        });
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let expected: Config = Config {
            general: GeneralConfig {
                sockfile: "/tmp/supervisord.sock".to_owned(),
                pidfile: "/tmp/supervisord.pid".to_owned(),
            },
            programs: Default::default(),
        };
        let c = Config::from("./src/lib/config/test/general_no_option.ini");
        assert_eq!(expected, c.unwrap());
    }

    #[test]
    fn test_general() {
        let expected: Config = Config {
            general: GeneralConfig {
                sockfile: "/tmp/test.general.sock".to_owned(),
                pidfile: "/tmp/test.general.pid".to_owned(),
            },
            programs: Default::default(),
        };
        let c = Config::from("./src/lib/config/test/general.ini");
        assert_eq!(expected, c.unwrap());
    }

    #[test]
    fn test_general_no_pidfile() {
        let expected: Config = Config {
            general: GeneralConfig {
                sockfile: "/tmp/test.general.sock".to_owned(),
                pidfile: "/tmp/supervisord.pid".to_owned(),
            },
            programs: Default::default(),
        };
        let c = Config::from("./src/lib/config/test/general_no_pidfile.ini");
        assert_eq!(expected, c.unwrap());
    }

    #[test]
    fn test_general_no_sockfile() {
        let expected: Config = Config {
            general: GeneralConfig {
                sockfile: "/tmp/supervisord.sock".to_owned(),
                pidfile: "/tmp/test.general.pid".to_owned(),
            },
            programs: Default::default(),
        };
        let c = Config::from("./src/lib/config/test/general_no_sockfile.ini");
        assert_eq!(expected, c.unwrap());
    }

    #[test]
    fn test_general_no_option() {
        let expected: Config = Config {
            general: GeneralConfig {
                sockfile: "/tmp/supervisord.sock".to_owned(),
                pidfile: "/tmp/supervisord.pid".to_owned(),
            },
            programs: Default::default(),
        };
        let c = Config::from("./src/lib/config/test/general_no_option.ini");
        assert_eq!(expected, c.unwrap());
    }

    #[test]
    fn test_general_invalid_key() {
        let c = Config::from("./src/lib/config/test/general_invalid_key.ini");
        assert_eq!(
            "configuration: invalid key: sock",
            c.unwrap_err().to_string()
        );
    }

    #[test]
    fn test_program_invalid_value_u32() {
        let c = Config::from("./src/lib/config/test/program_invalid_value_u32.ini");
        assert_eq!(
            "configuration: invalid value: numprocs: three",
            c.unwrap_err().to_string()
        );
    }

    #[test]
    fn test_program_invalid_value_bool() {
        let c = Config::from("./src/lib/config/test/program_invalid_value_bool.ini");
        assert_eq!(
            "configuration: invalid value: autostart: ff",
            c.unwrap_err().to_string()
        );
    }

    #[test] // FIXME
    fn test_program_invalid_value_umask() {
        let c = Config::from("./src/lib/config/test/program_invalid_value_umask.ini");
        assert_eq!(
            "configuration: invalid value: umask: asdf",
            c.unwrap_err().to_string()
        );
    }

    #[test]
    fn test_program_invalid_value_exitcode() {
        let c = Config::from("./src/lib/config/test/program_invalid_value_exitcode.ini");
        assert_eq!(
            "configuration: invalid value: exitcodes: asdf",
            c.unwrap_err().to_string()
        );
    }
    #[test]
    fn test_program_invalid_value_environment() {
        let c = Config::from("./src/lib/config/test/program_invalid_value_environment.ini");
        assert_eq!(
            "configuration: invalid value: environment: A=1,B",
            c.unwrap_err().to_string()
        );
    }

    #[test]
    fn test_program_invalid_value_stopsignal() {
        let c = Config::from("./src/lib/config/test/program_invalid_value_stopsignal.ini");
        assert_eq!(
            "configuration: invalid value: stopsignal: asdf",
            c.unwrap_err().to_string()
        );
    }

    #[test]
    fn test_program_invalid_value_autorestart() {
        let c = Config::from("./src/lib/config/test/program_invalid_value_autorestart.ini");
        assert_eq!(
            "configuration: invalid value: autorestart: asdf",
            c.unwrap_err().to_string()
        );
    }

    #[test]
    fn test_program_invalid_key() {
        let c = Config::from("./src/lib/config/test/program_invalid_key.ini");
        assert_eq!(
            "configuration: invalid key: sock",
            c.unwrap_err().to_string()
        );
    }

    #[test]
    fn test_program_no_command() {
        let c = Config::from("./src/lib/config/test/program_no_command.ini");
        assert_eq!(
            "configuration: there is no command in program",
            c.unwrap_err().to_string()
        );
    }

    #[test]
    fn test_program() {
        let mut expected = Config {
            general: GeneralConfig::new(),
            programs: HashMap::new(),
        };

        expected
            .programs
            .insert("a".to_owned(), ProgramConfig::new("a"));

        let program_config = expected.programs.get_mut("a").unwrap();
        program_config.command.push("/bin/ls".to_owned());
        program_config
            .environment
            .insert("A".to_owned(), "1".to_owned());
        program_config
            .environment
            .insert("B".to_owned(), "2".to_owned());
        program_config.exitcodes.pop();
        program_config.exitcodes.push(1);
        program_config.exitcodes.push(2);
        program_config.exitcodes.push(3);
        program_config.umask = Some(146);
        program_config.numprocs = 3;
        program_config.autostart = false;
        program_config.autorestart = AutoRestart::Never;
        program_config.stopsignal = Signal::SIGKILL;

        let c = Config::from("./src/lib/config/test/program.ini");
        assert_eq!(expected, c.unwrap())
    }
}
