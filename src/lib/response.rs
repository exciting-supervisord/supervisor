use serde::{Deserialize, Serialize};

// pub type Response = Result<OutputMessage, Error>;

#[derive(Deserialize, Serialize)]
pub struct Response {
    pub list: Vec<Result<OutputMessage, Error>>,
}

impl Response {
    pub fn new() -> Self {
        Response { list: Vec::new() }
    }

    pub fn add(&mut self, res: Result<OutputMessage, Error>) {
        self.list.push(res);
    }

    pub fn from_err(err: Error) -> Self {
        let mut res = Response::new();
        res.add(Err(err));
        res
    }

    pub fn from_output(out: OutputMessage) -> Self {
        let mut res = Response::new();
        res.add(Ok(out));
        res
    }
}

impl FromIterator<Result<OutputMessage, Error>> for Response {
    fn from_iter<T: IntoIterator<Item = Result<OutputMessage, Error>>>(iter: T) -> Self {
        let mut res = Response::new();

        for i in iter {
            res.list.push(i);
        }
        res
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Error {
    FileFormat(String),
    FileOpenError(String),
    Service(String),
    ProcessNotFound(String),
    ProcessNotRunning(String),
    ProcessAlreadyStarted(String),
    ProcessSpawnError(String),
}

impl Error {
    pub fn file_format(s: &str) -> Self {
        Error::FileFormat(s.to_owned())
    }

    pub fn file_open(s: &str) -> Self {
        Error::FileOpenError(s.to_owned())
    }

    pub fn service(s: &str) -> Self {
        Error::Service(s.to_owned())
    }

    pub fn spawn(s: &str) -> Self {
        Error::ProcessSpawnError(s.to_owned())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FileFormat(ref s) => write!(f, "{s}: Invalid configuraion file."),
            Error::FileOpenError(ref s) => write!(f, "{s}: can not open file."),
            Error::Service(ref s) => write!(f, "{s}: Service not available."),
            Error::ProcessNotFound(ref s) => write!(f, "{s}: no such process."),
            Error::ProcessNotRunning(ref s) => write!(f, "{s}: not running."),
            Error::ProcessAlreadyStarted(ref s) => write!(f, "{s}: already started."),
            Error::ProcessSpawnError(ref s) => write!(f, "{s}: can not spawn process."),
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
