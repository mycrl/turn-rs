// use.
use tokio::net::{ TcpListener, tcp::Incoming };
use std::net::SocketAddr;
use std::error::Error;
use futures::prelude::*;
use futures::try_ready;
use crate::stream::Socket;
use crate::shared::Shared;
use crate::bytes_stream::BytesStream;
use std::sync::Arc;
use std::sync::Mutex;


pub struct Server {
    pub address: SocketAddr,
    pub listener: Incoming,
    pub shared: Arc<Mutex<Shared>>
}


impl Server {
    pub fn new (addr: &'static str, shared: Arc<Mutex<Shared>>) -> Result<Self, Box<dyn Error>> {
        let address = addr.parse()?;
        let listener = TcpListener::bind(&address)?.incoming();
        Ok(Server { address, listener, shared })
    }
}


impl Future for Server {
    type Item = ();
    type Error = ();

    fn poll (&mut self) -> Poll<Self::Item, Self::Error> {
        while let Some(stream) = try_ready!(self.listener.poll().map_err(drop)) {
            let bytes_stream = BytesStream::new(stream);
            tokio::spawn(Socket::new(bytes_stream, self.shared.clone()));
        }

        Ok(Async::Ready(()))
    }
}