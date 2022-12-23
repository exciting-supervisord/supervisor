extern crate libc;

pub use libc::termios;

pub trait Constructor {
    fn new() -> Self;
}

impl Constructor for termios {
    fn new() -> Self {
        termios {
            c_iflag: 0,
            c_oflag: 0,
            c_ispeed: 0,
            c_ospeed: 0,
            c_cflag: 0,
            c_lflag: 0,
            c_cc: Default::default(),
        }
    }
}
