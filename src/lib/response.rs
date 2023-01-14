use super::process_status::ProcessStatus;
use serde::{Deserialize, Serialize};

pub type CommandResult = Result<RpcOutput, RpcError>;

#[derive(Deserialize, Serialize, Debug)]
pub enum Response {
    Command(Vec<CommandResult>),
    Status(Vec<ProcessStatus>),
}

impl Response {
    pub fn from_output(out: RpcOutput) -> Self {
        let mut res = Vec::new();
        res.push(Ok(out));
        Response::Command(res)
    }

    pub fn from_err(err: RpcError) -> Self {
        let mut res = Vec::new();
        res.push(Err(err));
        Response::Command(res)
    }
}

impl std::ops::Add for Response {
    type Output = Response;
    fn add(mut self, rhs: Self) -> Self::Output {
        match self {
            Response::Command(ref mut v1) => match rhs {
                Response::Command(mut v2) => v1.append(&mut v2),
                Response::Status(_) => panic!("logic error"),
            },
            Response::Status(ref mut v1) => match rhs {
                Response::Command(_) => panic!("logic error"),
                Response::Status(mut v2) => v1.append(&mut v2),
            },
        }
        self
    }
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Response::Command(ref v) => {
                v.iter().for_each(|cmd_res| match *cmd_res {
                    Ok(ref o) => write!(f, "{o}\n").unwrap_or_default(),
                    Err(ref e) => write!(f, "{e}\n").unwrap_or_default(),
                });
                Ok(())
            }
            Response::Status(ref v) => {
                v.iter().for_each(|status| {
                    write!(f, "{}\n", status).unwrap_or_default();
                });
                Ok(())
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum RpcError {
    FileFormat(String),
    FileOpenError(String),
    Service(String),
    InvalidRequest(String),
    ProcessNotFound(String),
    ProcessNotRunning(String),
    ProcessAlreadyStarted(String),
    ProcessSpawnError(String),
}

impl RpcError {
    pub fn file_format(s: &str) -> Self {
        RpcError::FileFormat(s.to_owned())
    }

    pub fn file_open(s: &str) -> Self {
        RpcError::FileOpenError(s.to_owned())
    }

    pub fn service(s: &str) -> Self {
        RpcError::Service(s.to_owned())
    }

    pub fn invalid_request(s: &str) -> Self {
        RpcError::InvalidRequest(s.to_owned())
    }

    pub fn spawn(s: &str) -> Self {
        RpcError::ProcessSpawnError(s.to_owned())
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::FileFormat(ref s) => write!(f, "{s}: Invalid configuraion file."),
            RpcError::FileOpenError(ref s) => write!(f, "{s}: can not open file."),
            RpcError::Service(ref s) => write!(f, "{s}: Service not available."),
            RpcError::InvalidRequest(ref s) => write!(f, "Invalid Request: {s}"),
            RpcError::ProcessNotFound(ref s) => write!(f, "{s}: no such process."),
            RpcError::ProcessNotRunning(ref s) => write!(f, "{s}: not running."),
            RpcError::ProcessAlreadyStarted(ref s) => write!(f, "{s}: already started."),
            RpcError::ProcessSpawnError(ref s) => write!(f, "{s}: can not spawn process."),
        }
    }
}

impl std::error::Error for RpcError {}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcOutput {
    name: String,
    message: String,
}

impl RpcOutput {
    pub fn new(name: &str, message: &str) -> Self {
        RpcOutput {
            name: name.to_string(),
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for RpcOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.message)
    }
}
