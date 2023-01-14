extern crate nix;

pub mod autorestart;
pub mod process_config;
pub mod program_config;
mod config_error;
mod general_config;
mod parser_ini;

use general_config::*;
use program_config::*;

use super::process_id::ProcessId;

use std::collections::{HashMap, HashSet};
use std::error::Error;

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
    use autorestart::*;
    use nix::sys::signal::Signal;

    #[test]
    fn test_empty() {
        let expected: Config = Config {
            general: GeneralConfig {
                sockfile: "/tmp/taskmasterd.sock".to_owned(),
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
            },
            programs: Default::default(),
        };
        let c = Config::from("./src/lib/config/test/general.ini");
        assert_eq!(expected, c.unwrap());
    }

    #[test]
    fn test_general_no_option() {
        let expected: Config = Config {
            general: GeneralConfig {
                sockfile: "/tmp/taskmasterd.sock".to_owned(),
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

    #[test]
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
