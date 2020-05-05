extern crate bytes;
extern crate rml_rtmp;
extern crate tokio;

mod codec;
mod server;

use server::Server;
use std::error::Error;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "0.0.0.0:1935".parse().unwrap();
    let mut server = Server::new(addr).await?;
    
    loop {
        server.next().await;
    }
}
