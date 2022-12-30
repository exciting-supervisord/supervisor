use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Request {
    method: String,
    args: Vec<String>,
}
