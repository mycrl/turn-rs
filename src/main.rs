extern crate bytes;
extern crate actix;
extern crate tokio;
extern crate rml_rtmp;


// mod.
mod stream;
mod shared;
mod handshake;
mod session;
mod server;
mod bytes_stream;


// use.
use futures::sync::mpsc;
use std::error::Error;
use server::Server;
use stream::Message;
use shared::Shared;
use std::sync::Arc;
use std::sync::Mutex;


// type.
pub type Tx = mpsc::UnboundedSender<Message>;
pub type Rx = mpsc::UnboundedReceiver<Message>;


fn main() -> Result<(), Box<dyn Error>> {
    let shared = Arc::new(Mutex::new(Shared::new()));
    tokio::run(Server::new("0.0.0.0:1935", shared)?);
    Ok(())
}