mod peer;
mod server;

use futures::prelude::*;
use std::error::Error;
use server::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "0.0.0.0:1936".parse().unwrap();
    let mut server = Server::new(addr).await?;
    loop {
        server.next().await;
    }
}
