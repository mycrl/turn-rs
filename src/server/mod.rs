pub mod socket;

use futures::prelude::*;
use socket::Socket;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub struct Server(TcpListener);

impl Server {
    pub fn new(addr: SocketAddr) -> Result<Self, Box<dyn Error>> {
        Ok(Self(TcpListener::bind(&addr)?))
    }
}

impl Future for Server {
    type Item = ();
    type Error = ();

    #[rustfmt::skip]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Ok(Async::Ready((socket, _))) = self.0.poll_accept() {
            tokio::spawn(Socket::new(socket));
        }

        Ok(Async::NotReady)
    }
}
