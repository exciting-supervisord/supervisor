use std::sync::atomic::{AtomicBool, Ordering};

use libc::{SIGHUP, SIGINT, SIGTERM};
use nix::sys::signal::{self, SigHandler, Signal};

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);
pub static UPDATE: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_termination(signal: libc::c_int) {
    SHUTDOWN.store(SIGINT == signal || SIGTERM == signal, Ordering::Relaxed);
}

extern "C" fn handle_update(signal: libc::c_int) {
    UPDATE.store(SIGHUP == signal, Ordering::Relaxed);
}

pub fn set_signal_handlers() {
    let term_handler = SigHandler::Handler(handle_termination);
    let hup_handler = SigHandler::Handler(handle_update);
    unsafe {
        signal::signal(Signal::SIGINT, term_handler).expect("signal SIGINT");
        signal::signal(Signal::SIGTERM, term_handler).expect("signal SIGTERM");
        signal::signal(Signal::SIGHUP, hup_handler).expect("signal SIGHUP");
    }
}
