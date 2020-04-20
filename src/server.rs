use crate::socket::Socket;
use futures::prelude::*;
use futures::try_ready;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::tcp::Incoming;
use tokio::net::TcpListener;

pub struct Server {
    pub listener: Incoming,
}

impl Server {
    pub fn new(addr: SocketAddr) -> Result<Self, Box<dyn Error>> {
        Ok(Server {
            listener: TcpListener::bind(&addr)?.incoming(),
        })
    }
}

impl Future for Server {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Some(stream) = try_ready!(self.listener.poll().map_err(drop)) {
            tokio::spawn(Socket::new(stream));
        }

        Ok(Async::Ready(()))
    }
}
