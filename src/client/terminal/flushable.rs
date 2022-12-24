use std::io::{self, Write};

pub trait Flushable {
    fn flush_stdout(&self) {
        if let Err(e) = io::stdout().flush() {
            panic!("{e}");
        }
    }
}
