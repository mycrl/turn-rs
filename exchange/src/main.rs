mod router;
mod server;

use std::io::Error;
use configure::Configure;

#[tokio::main]
async fn main() -> Result<(), Error> {
    balance::start("0.0.0.0:8088".parse().unwrap())?;
    let configure = Configure::generate();
    server::run(configure).await
}
