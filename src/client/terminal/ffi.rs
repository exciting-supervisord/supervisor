use super::termios_ctor::*;

use nix::{
    errno,
    sys::termios::{
        tcgetattr, tcsetattr, LocalFlags, SetArg::*, SpecialCharacterIndices::*, Termios,
    },
    unistd::read,
};

pub fn getch() -> Result<u8, errno::Errno> {
    let t = termios::new();
    let oterm = tcgetattr(0)?;
    let mut term = Termios::from(t);

    term = oterm.clone();
    term.local_flags &= !(LocalFlags::ICANON | LocalFlags::ECHO);
    term.control_chars[VMIN as usize] = 1;
    term.control_chars[VTIME as usize] = 0;

    tcsetattr(0, TCSANOW, &term)?;
    let mut c: [u8; 1] = [0];
    read(0, &mut c)?;
    tcsetattr(0, TCSANOW, &oterm)?;
    Ok(c[0] as u8)
}
