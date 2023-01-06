mod control;
mod net;
mod supervisor;

use lib::config::Config;
use lib::daemon::daemonize;
use lib::logger::LOG;
use lib::{CONF_FILE, LOG_FILE};

use net::UdsRpcServer;
use supervisor::Supervisor;

use core::cell::RefCell;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

fn set_command_handlers<'a, 'b>(
    server: &'a mut UdsRpcServer<'b>,
    supervisor: &'b RefCell<Supervisor>,
) {
    let status = |args| supervisor.borrow_mut().status(args);
    let start = |args| supervisor.borrow_mut().start(args);
    let stop = |args| supervisor.borrow_mut().stop(args);
    let shutdown = |args| supervisor.borrow_mut().shutdown(args);
    let reload = |args| supervisor.borrow_mut().reload(args);
    let update = |args| supervisor.borrow_mut().update(args);

    server.add_method("status", status);
    server.add_method("start", start);
    server.add_method("stop", stop);
    server.add_method("shutdown", shutdown);
    server.add_method("reload", reload);
    server.add_method("update", update);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    control::set_signal_handlers();
    daemonize(LOG_FILE).unwrap_or_else(|e| lib::exit_with_error(Box::new(e)));

    LOG.info(&format!("read config file from {CONF_FILE}"));
    let conf = Config::from(CONF_FILE).unwrap_or_else(|e| lib::exit_with_error(e));

    let supervisor = Supervisor::new(CONF_FILE, conf)?;
    let supervisor = RefCell::new(supervisor);
    let mut server = UdsRpcServer::new(supervisor.borrow().sockfile())
        .unwrap_or_else(|e| lib::exit_with_error(e));
    LOG.info(&format!(
        "RPC server listen at {}",
        supervisor.borrow().sockfile()
    ));

    set_command_handlers(&mut server, &supervisor);

    loop {
        server.try_handle_client();
        supervisor.borrow_mut().supervise()?;

        thread::sleep(Duration::from_millis(500));
        if control::UPDATE.load(Ordering::Relaxed) {
            supervisor.borrow_mut().update(Vec::new());
            control::UPDATE.store(false, Ordering::Relaxed);
        }
        if control::SHUTDOWN.load(Ordering::Relaxed) {
            LOG.info("shutdown signal dected.. cleaning up"); // sockfile ?
            supervisor.borrow_mut().cleanup_processes();

            break;
        }
    }
    Ok(())
}
