pub enum Error {
    FileFormat,
    Service,
    ProcessNotFound(String),
    ProcessNotRunning(String),
    ProcessAlreadyStarted(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FileFormat => write!(f, "Invalid configuraion file."),
            Error::Service => write!(f, "Service temporary unavailable."),
            Error::ProcessNotFound(ref s) => write!(f, "{s}: no such process "),
            Error::ProcessNotRunning(ref s) => write!(f, "{s}: not running"),
            Error::ProcessAlreadyStarted(ref s) => write!(f, "{s}: already started"),
        }
    }
}
