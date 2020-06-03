#[macro_use]
extern crate lazy_static;

mod server;
mod codec;

use std::error::Error;
use configure::Configure;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let configure = Configure::generate();
    Ok(server::run(configure).await?)
}
