mod server;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    balance::start("0.0.0.0:8088".parse().unwrap())?;
    Ok(server::run().await?)
}
