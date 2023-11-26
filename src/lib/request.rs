use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::response::Response;

pub type Procedure<A> = Arc<dyn Fn(A) -> Response + Sync + Send>;

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

pub struct ReqMethod<A> {
    method: Procedure<A>,
    args: A,
}

impl<A> ReqMethod<A> {
    pub fn new(procedure: Procedure<A>, args: A) -> Self {
        Self {
            method: procedure,
            args,
        }
    }

    pub fn run(self) -> Response {
        let Self { method, args } = self;
        method(args)
    }
}
