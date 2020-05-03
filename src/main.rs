mod rtmp;
mod server;

use std::error::Error;
use futures::StreamExt;
use server::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "0.0.0.0:1935".parse().unwrap();
    let mut server = Server::new(addr).await?;

    loop {
        server.next().await;
    }
}
