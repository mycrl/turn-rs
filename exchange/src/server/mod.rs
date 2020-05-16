pub mod socket;

use futures::prelude::*;
use std::pin::Pin;
use std::error::Error;
use std::net::SocketAddr;
use std::task::{Context, Poll};
use tokio::net::TcpListener;

pub struct Server {
    tcp: TcpListener,
}

impl Server {
    /// Create a TCP server.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::Server;
    /// use tokio::sync::mpsc;
    ///
    /// let addr = "0.0.0.0:8080".parse().unwrap();
    /// let (sender, _) = mpsc::unbounded_channel();
    ///
    /// Server::new(addr, sender).await.unwrap();
    /// ```
    pub async fn new(addr: SocketAddr) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            tcp: TcpListener::bind(&addr).await?,
        })
    }
}

impl Stream for Server {
    type Item = Result<(), Box<dyn Error>>;

    #[rustfmt::skip]
    fn poll_next (self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        let handle = self.get_mut();
        match handle.tcp.poll_accept(ctx) {
            Poll::Ready(Ok((socket, _))) => {
                
                Poll::Ready(Some(Ok(())))
            }, _ => Poll::Pending
        }
    }
}
