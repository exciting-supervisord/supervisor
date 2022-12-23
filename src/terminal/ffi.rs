use super::termios_ctor::*;

use libc::c_void;
use libc::{getchar, memcpy, tcgetattr, tcsetattr};
use libc::{ECHO, ICANON, TCSANOW, VMIN, VTIME};

pub fn getch() -> u8 {
    let mut term = termios::new();
    let mut oterm = termios::new();
    let termp = &mut term as *mut termios;
    let otermp = &mut oterm as *mut termios;
    let size = std::mem::size_of::<termios>();

    unsafe {
        tcgetattr(0, otermp);
        memcpy(termp as *mut c_void, otermp as *mut c_void, size);
        term.c_lflag &= !(ICANON | ECHO);
        term.c_cc[VMIN] = 1;
        term.c_cc[VTIME] = 0;
        tcsetattr(0, TCSANOW, termp);
        let c = getchar();
        tcsetattr(0, TCSANOW, otermp);
        c as u8
    }
}
