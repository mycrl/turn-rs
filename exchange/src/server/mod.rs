mod socket;

use futures::prelude::*;
use std::net::SocketAddr;
use std::{io::Error, pin::Pin};
use std::task::{Context, Poll};
use tokio::net::TcpListener;

/// TcpServer
pub struct Server {
    listener: TcpListener
}

impl Server {
    pub async fn new(addr: SocketAddr) -> Result<Self, Error> {
        Ok(Self {
            listener: TcpListener::bind(addr).await?,
        })
    }
}

impl Stream for Server {
    type Item = Result<(), Error>;

    #[rustfmt::skip]
    fn poll_next (self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        let handle = self.get_mut();
        match handle.listener.poll_accept(ctx) {
            Poll::Ready(Ok((socket, _))) => {
                Poll::Ready(Some(Ok(())))
            }, _ => Poll::Pending
        }
    }
}
