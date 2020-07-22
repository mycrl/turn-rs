mod router;
mod server;

use std::io::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    balance::start("0.0.0.0:8088".parse().unwrap())?;
    server::run().await
}
