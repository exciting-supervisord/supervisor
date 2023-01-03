use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Request {
    pub method: String,
    pub args: Vec<String>,
}

impl Request {
    pub fn from(words: &Vec<&str>) -> Self {
        let mut args: Vec<String> = Default::default();
        words.iter().skip(1).for_each(|x| args.push(x.to_string()));
        Request {
            method: words[0].to_owned(),
            args,
        }
    }
}
