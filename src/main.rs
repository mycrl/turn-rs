extern crate bytes;
extern crate rml_rtmp;
extern crate tokio;
#[macro_use]
extern crate lazy_static;

mod codec;
mod server;

use futures::StreamExt;
use server::Server;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "0.0.0.0:1935".parse().unwrap();
    let mut server = Server::new(addr).await?;

    loop {
        server.next().await;
    }
}
