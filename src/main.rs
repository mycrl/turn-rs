mod rtmp;
mod server;

use std::error::Error;
use server::Server;

fn main() -> Result<(), Box<dyn Error>> {
    let addr = "0.0.0.0:1935".parse().unwrap();
    tokio::run(Server::new(addr)?);
    Ok(())
}
