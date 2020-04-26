extern crate bytes;
extern crate rml_rtmp;
extern crate tokio;

mod rtmp;
mod server;

use server::Server;
use std::io::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let addr = "0.0.0.0:1935".parse().unwrap();
    Server::new(addr).await;
    Ok(())
}
