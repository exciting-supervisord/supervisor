use lib::CONF_FILE;
use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct ArgError {
    prog_name: String,
}

impl ArgError {
    pub fn new(name: &str) -> Self {
        Self {
            prog_name: name.to_string(),
        }
    }
}

impl Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "usage: {} [conf_file]", self.prog_name)?;
        write!(
            f,
            "if conf_file is missing, default ({CONF_FILE}) will be used."
        )
    }
}

impl Error for ArgError {}
