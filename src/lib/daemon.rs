use libc;
use nix::{
    errno::Errno,
    sys::stat::Mode,
    unistd::{chdir, close, dup2, fork, setsid, ForkResult},
};

use nix::fcntl::{open, OFlag};

fn replace_std_fd(filename: &str) -> Result<(), Errno> {
    let fd = open(
        filename,
        OFlag::O_RDWR | OFlag::O_APPEND | OFlag::O_CREAT,
        Mode::from_bits(0o600).expect("hardcoded mode"),
    )?;

    dup2(fd, 0)?;
    dup2(fd, 1)?;
    dup2(fd, 2)?;

    close(fd)
}

pub fn daemonize(logfile: &str) -> Result<(), Errno> {
    let proc = unsafe { fork() }?;

    if let ForkResult::Parent { .. } = proc {
        unsafe { libc::_exit(0) };
    }

    setsid()?;
    replace_std_fd(logfile)?;
    // chdir("/tmp")

    Ok(())
}
