mod server;

use std::io::Error;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let addr = "0.0.0.0:8088".parse::<SocketAddr>().unwrap();
    server::run(addr).await
}
