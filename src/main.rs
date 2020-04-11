extern crate bytes;
extern crate tokio;
extern crate rml_rtmp;

// mod stream;
// mod shared;
// mod handshake;
// mod session;
mod server;
mod socket;
mod handshake;

// use futures::sync::mpsc;
use std::error::Error;
use server::Server;
// use stream::Message;

// pub type Tx = mpsc::UnboundedSender<Message>;
// pub type Rx = mpsc::UnboundedReceiver<Message>;

fn main() -> Result<(), Box<dyn Error>> {
    tokio::run(Server::new("0.0.0.0:1935".parse()?)?);
    Ok(())
}
