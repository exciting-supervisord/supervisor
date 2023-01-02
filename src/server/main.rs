pub mod core;

use jsonrpc_ipc_server::jsonrpc_core::{IoHandler, Params, Value};
use jsonrpc_ipc_server::ServerBuilder;
use lib::config::Config;

use std::error::Error;

const CONF_FILE: &'static str = "./general.ini";

fn main() -> Result<(), Box<dyn Error>> {
    let conf = Config::from(CONF_FILE)?;

    let mut io = IoHandler::default();
    io.add_method("add", |_params: Params| async {
        Ok(Value::String("hello".to_owned()))
    });

    let server = ServerBuilder::new(io).start(&conf.general.sockfile)?;

    server.wait();

    Ok(())
}
