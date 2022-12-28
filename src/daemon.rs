use libc;
use nix::{
    errno::Errno,
    sys::stat::Mode,
    unistd::{close, dup2, fork, setsid, ForkResult},
};

use nix::fcntl::{open, OFlag};

fn replace_std_fd(filename: &str) -> Result<(), Errno> {
    let fd = open(filename, OFlag::O_RDWR, Mode::all())?;

    dup2(fd, 0)?;
    dup2(fd, 1)?;
    dup2(fd, 2)?;

    close(fd)
}

// TODO double fork, clear env
pub fn daemonize() -> Result<(), Errno> {
    match unsafe { fork() } {
        Ok(ForkResult::Parent { .. }) => unsafe {
            libc::_exit(0);
        },
        Ok(ForkResult::Child) => setsid(),
        Err(e) => Err(e),
    }?;
    replace_std_fd("/dev/null")
}
