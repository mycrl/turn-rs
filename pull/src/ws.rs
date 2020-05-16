use futures::prelude::*;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::net::TcpStream;
use std::task::{Context, Poll};
use tungstenite::{
    server::accept, 
    protocol::WebSocket
};

pub struct Ws {
    ws: WebSocket<TcpStream>
}

impl Ws {
    pub fn new(stream: TcpStream) -> Result<Self, Error> {
        match accept(stream) {
            Ok(ws) => Ok(Self { ws }),
            _ => Err(Error::new(ErrorKind::NotConnected, ""))
        }
    }
}

impl Future for Ws {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, _: &mut Context) -> Poll<Self::Output> {
        Poll::Pending
    }
}
