use serde::{Deserialize, Serialize};

use super::process_status::ProcessStatus;

#[derive(Deserialize, Serialize)]
pub enum Response {
    Action(Action),
    Status(Vec<ProcessStatus>),
}

impl Response {
    pub fn from_output(out: OutputMessage) -> Self {
        let mut res = Action::new();
        res.add(Ok(out));
        Response::Action(res)
    }

    pub fn from_err(err: Error) -> Self {
        let mut res = Action::new();
        res.add(Err(err));
        Response::Action(res)
    }
}

#[derive(Deserialize, Serialize)]
pub struct Action {
    pub list: Vec<Result<OutputMessage, Error>>,
}

impl Action {
    pub fn new() -> Self {
        Action { list: Vec::new() }
    }

    pub fn add(&mut self, res: Result<OutputMessage, Error>) {
        self.list.push(res);
    }

    // pub fn from_err(err: Error) -> Self {
    //     let mut res = Action::new();
    //     res.add(Err(err));
    //     res
    // }

    // pub fn from_output(out: OutputMessage) -> Self {
    //     let mut res = Action::new();
    //     res.add(Ok(out));
    //     res
    // }
}

impl FromIterator<Result<OutputMessage, Error>> for Action {
    fn from_iter<T: IntoIterator<Item = Result<OutputMessage, Error>>>(iter: T) -> Self {
        let mut res = Action::new();

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
