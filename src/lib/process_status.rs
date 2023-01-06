use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Copy, Deserialize, Serialize, Debug)]
pub enum ProcessState {
    Stopped,
    Starting,
    Running,
    Backoff,
    Stopping,
    Exited,
    Fatal,
    Unknown,
}

impl ProcessState {
    pub fn stopable(&self) -> bool {
        *self != ProcessState::Stopped
            && *self != ProcessState::Stopping
            && *self != ProcessState::Fatal
            && *self != ProcessState::Exited
    }

    pub fn startable(&self) -> bool {
        *self != ProcessState::Starting
            && *self != ProcessState::Backoff
            && *self != ProcessState::Running
    }
}

impl ToString for ProcessState {
    fn to_string(&self) -> String {
        match self {
            Self::Stopped => format!("Stopped"),
            Self::Starting => format!("Starting"),
            Self::Running => format!("Running"),
            Self::Backoff => format!("Backoff"),
            Self::Stopping => format!("Stopping"),
            Self::Exited => format!("Exited"),
            Self::Fatal => format!("Fatal"),
            Self::Unknown => format!("Unknown"),
        }
    }
}

// impl std::fmt::Display for ProcessState {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Stopped => write!(f, "Stopped"),
//             Self::Starting => write!(f, "Starting"),
//             Self::Running => write!(f, "Running"),
//             Self::Backoff => write!(f, "Backoff"),
//             Self::Stopping => write!(f, "Stopping"),
//             Self::Exited => write!(f, "Exited"),
//             Self::Fatal => write!(f, "Fatal"),
//             Self::Unknown => write!(f, "Unknown"),
//         }
//     }
// }

#[derive(Deserialize, Serialize, Debug)]
pub struct ProcessStatus {
    name: String,
    seq: u32,
    state: ProcessState,
    description: String,
}

impl std::fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:<30}{:<20}{}",
            format!("{}:{}", self.name, self.seq),
            self.state.to_string(),
            self.description
        )
    }
}

impl ProcessStatus {
    pub fn new(name: String, seq: u32, state: ProcessState, description: String) -> Self {
        ProcessStatus {
            name,
            seq,
            state,
            description,
        }
    }
}
