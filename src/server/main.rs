mod control;
mod error;
mod net;
mod supervisor;

use error::ArgError;
use lib::config::Config;
use lib::daemon::daemonize;
use lib::logger::LOG;
use lib::{CONF_FILE, LOG_FILE};

use net::UdsRpcServer;
use std::env;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use supervisor::SupvArg;

fn init() -> Result<Arc<UdsRpcServer<SupvArg>>, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let conf_file = match args.len() {
        1 => Ok(CONF_FILE),
        2 => Ok(args[1].as_str()),
        _ => Err(Box::new(ArgError::new(&args[0]))),
    }?;

    control::set_signal_handlers();
    daemonize(LOG_FILE)?;

    LOG.info(&format!("read config file from {conf_file}"));
    let conf = Config::from(conf_file)?;
    let sock_file = conf.general.sockfile.to_string();

    let mut server = UdsRpcServer::new(&sock_file)?;
    LOG.info(&format!("RPC server listen at {}", sock_file));

    supervisor::init(conf_file, conf)?;
    supervisor::register_rpc(&mut server);

    Ok(Arc::new(server))
}

fn main() {
    let server = match init() {
        Err(e) => lib::exit_with_log(e),
        Ok(server) => server,
    };

    loop {
        server.accept_client();

        if let Err(e) = supervisor::supervise() {
            lib::exit_with_log(e);
        }

        if control::UPDATE.load(Ordering::Acquire) {
            LOG.info("reload signal (HUP) detected.. reloading configuration.");
            supervisor::update();
            control::UPDATE.store(false, Ordering::Release);
        }

        if control::SHUTDOWN.load(Ordering::Relaxed) {
            LOG.info("shutdown signal detected.. cleaning up");
            supervisor::cleanup_processes();

            break;
        }
    }
}
