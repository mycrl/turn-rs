extern crate bytes;
extern crate rml_rtmp;
extern crate tokio;

// mod stream;
// mod shared;
// mod handshake;
// mod session;
mod handshake;
mod server;
mod socket;
mod session;
mod rtmp;

// use futures::sync::mpsc;
use server::Server;
use std::error::Error;
// use stream::Message;

// pub type Tx = mpsc::UnboundedSender<Message>;
// pub type Rx = mpsc::UnboundedReceiver<Message>;

fn main() -> Result<(), Box<dyn Error>> {
    tokio::run(Server::new("0.0.0.0:1935".parse()?)?);
    Ok(())
}
