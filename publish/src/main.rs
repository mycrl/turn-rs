#[macro_use]
extern crate lazy_static;

mod server;
mod codec;

use std::error::Error;
use configure::Configure;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    balance::start("0.0.0.0:8088".parse().unwrap())?;
    let configure = Configure::generate();
    Ok(server::run(configure).await?)
}
