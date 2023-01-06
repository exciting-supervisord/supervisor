use std::error::Error;

#[derive(Debug)]
pub struct ConfigCommandError;

impl std::fmt::Display for ConfigCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "configuration: there is no command in program")
    }
}

impl Error for ConfigCommandError {}

#[derive(Debug)]
pub struct ConfigValueError {
    key: String,
    value: String,
}

impl ConfigValueError {
    pub fn new(key: &str, value: &str) -> Self {
        ConfigValueError {
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }
}

impl std::fmt::Display for ConfigValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "configuration: invalid value: {}: {}",
            self.key, self.value
        )
    }
}

impl Error for ConfigValueError {}

#[derive(Debug)]
pub struct ConfigKeyError(String);

impl ConfigKeyError {
    pub fn new(key: &str) -> Self {
        ConfigKeyError(key.to_owned())
    }
}

impl std::fmt::Display for ConfigKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "configuration: invalid key: {}", self.0)
    }
}

impl Error for ConfigKeyError {}

#[derive(Debug)]
pub enum ConfigFileError {
    Parsing(ConfigParsingError),
    Nofile(ConfigNoFileError),
}

impl std::fmt::Display for ConfigFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ConfigFileError::Parsing(_) => write!(f, "configuration: invalid file format"),
            ConfigFileError::Nofile(_) => write!(f, "configuration: no such file"),
        }
    }
}

impl Error for ConfigFileError {}

#[derive(Debug)]
pub struct ConfigParsingError;

#[derive(Debug)]
pub struct ConfigNoFileError;
