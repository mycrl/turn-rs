extern crate bytes;
extern crate tokio;
extern crate rml_rtmp;


// mod.
mod stream;
mod shared;
mod handshake;
mod session;
mod server;
mod socket;


// use.
use futures::sync::mpsc;
use std::error::Error;
use server::Server;
use stream::Message;


// type.
pub type Tx = mpsc::UnboundedSender<Message>;
pub type Rx = mpsc::UnboundedReceiver<Message>;


fn main() -> Result<(), Box<dyn Error>> {
    tokio::run(Server::new("0.0.0.0:1935".parse()?)?);
    Ok(())
}