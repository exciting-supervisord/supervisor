mod net;
mod supervisor;

use lib::config::Config;
use net::UdsRpcServer;
use supervisor::Supervisor;

use core::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use nix::sys::signal::{self, SigHandler, Signal};

const CONF_FILE: &'static str = "./general.ini";

static SIGNALED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigint(signal: libc::c_int) {
    let signal = Signal::try_from(signal).unwrap();
    SIGNALED.store(
        Signal::SIGINT == signal || Signal::SIGTERM == signal,
        Ordering::Relaxed,
    );
}

fn set_signal_handlers() {
    let handler = SigHandler::Handler(handle_sigint);
    unsafe {
        signal::signal(Signal::SIGINT, handler).expect("signal");
        signal::signal(Signal::SIGTERM, handler).expect("signal");
    }
}

fn set_command_handlers<'a, 'b>(
    server: &'a mut UdsRpcServer<'b>,
    supervisor: &'b RefCell<Supervisor>,
) {
    let start = |args| supervisor.borrow_mut().start(args);
    let stop = |args| supervisor.borrow_mut().stop(args);
    // let update = |args| supervisor.update(args);

    server.add_method("start", start);
    server.add_method("stop", stop);
    // server.add_method("update", update);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    set_signal_handlers();

    let conf = match Config::from(CONF_FILE) {
        Ok(o) => o,
        Err(e) => lib::exit_with_error(e),
    };

    let supervisor = Supervisor::new(CONF_FILE, conf)?;
    let supervisor = RefCell::new(supervisor);
    let mut server = match UdsRpcServer::new(supervisor.borrow().sockfile()) {
        Ok(o) => o,
        Err(e) => lib::exit_with_error(e),
    };

    set_command_handlers(&mut server, &supervisor); // 'a 'b

    loop {
        server.try_handle_client();
        supervisor.borrow_mut().supervise()?;

        thread::sleep(Duration::from_millis(100));
        println!("loop");
        if SIGNALED.load(Ordering::Relaxed) {
            println!("Signal detected. cleaning up...");
            break;
        }
    }
    Ok(())
}
