use serde::{Deserialize, Serialize};

pub type Response = Result<OutputMessage, Error>;

#[derive(Debug, Deserialize, Serialize)]
pub enum Error {
    FileFormat,
    Service(String),
    ProcessNotFound(String),
    ProcessNotRunning(String),
    ProcessAlreadyStarted(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FileFormat => write!(f, "Invalid configuraion file."),
            Error::Service(ref s) => write!(f, "{s}: Service not available."),
            Error::ProcessNotFound(ref s) => write!(f, "{s}: no such process."),
            Error::ProcessNotRunning(ref s) => write!(f, "{s}: not running."),
            Error::ProcessAlreadyStarted(ref s) => write!(f, "{s}: already started."),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Deserialize, Serialize)]
pub struct OutputMessage {
    name: String,
    message: String,
}

impl OutputMessage {
    pub fn new(name: &str, message: &str) -> Self {
        OutputMessage {
            name: name.to_string(),
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for OutputMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.message)
    }
}
