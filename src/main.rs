extern crate bytes;
extern crate rml_rtmp;
extern crate tokio;

mod codec;
mod server;

use std::error::Error;
use futures::StreamExt;
use server::Server;
use server::ServerAddress;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut server = Server::new(ServerAddress {
        tcp: "0.0.0.0:1935".parse().unwrap(),
        udp: "127.0.0.1:1936".parse().unwrap()
    }).await?;

    loop {
        server.next().await;
    }
}
