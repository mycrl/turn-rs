#[macro_use]
extern crate lazy_static;

mod codec;
mod server;

use server::ServerAddress;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    Ok(server::run(ServerAddress {
        tcp: "0.0.0.0:1935".parse().unwrap(),
        udp: "127.0.0.1:1936".parse().unwrap(),
    })
    .await?)
}
