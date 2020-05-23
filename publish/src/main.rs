mod codec;
mod server;

use server::ServerAddr;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    Ok(server::run(ServerAddr {
        consume: "0.0.0.0:1935".parse().unwrap(),
        produce: "127.0.0.1:1936".parse().unwrap(),
    })
    .await?)
}
