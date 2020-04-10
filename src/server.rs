use tokio::net::tcp::Incoming;
use tokio::net::TcpListener;
use std::net::SocketAddr;
use std::error::Error;
use futures::prelude::*;
use futures::try_ready;
use crate::socket::Socket;

pub struct Server {
    pub listener: Incoming,
}

impl Server {
    pub fn new (addr: SocketAddr) -> Result<Self, Box<dyn Error>> {
        Ok(Server { listener: TcpListener::bind(&addr)?.incoming() })
    }
}

impl Future for Server {
    type Item = ();
    type Error = ();
    fn poll (&mut self) -> Poll<Self::Item, Self::Error> {
        let result = self.listener.poll().unwrap();
        while let Some(stream) = try_ready!(result) {
            tokio::spawn(Socket::new(stream));
        }

        Ok(Async::Ready(()))
    }
}