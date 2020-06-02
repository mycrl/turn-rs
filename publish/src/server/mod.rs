pub mod forward;
pub mod socket;

use crate::codec::Rtmp;
use bytes::BytesMut;
use forward::Forward;
use futures::prelude::*;
use socket::Socket;
use std::error::Error;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use transport::Flag;

/// Byte stream read and write pipeline type.
pub type Tx = mpsc::UnboundedSender<(Flag, BytesMut)>;
pub type Rx = mpsc::UnboundedReceiver<(Flag, BytesMut)>;

/// Compound server address.
pub struct ServerAddr {
    pub consume: SocketAddr,
    pub produce: SocketAddr,
}

/// TCP Server.
///
/// Create a TCP server, bind to the specified port
/// address and process RTMP protocol messages.
///
/// # Examples
///
/// ```no_run
/// use server::Server;
/// use std::error::Error;
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     tokio::run(Server::new("0.0.0.0:1935".parse()?)?);
///     Ok(())
/// }
/// ```
pub struct Server {
    tcp: TcpListener,
    sender: Tx,
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
    /// let addr = "0.0.0.0:1935".parse().unwrap();
    /// let (sender, _) = mpsc::unbounded_channel();
    ///
    /// Server::new(addr, sender).await.unwrap();
    /// ```
    pub async fn new(addr: SocketAddr, sender: Tx) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            sender,
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
                tokio::spawn(Socket::<Rtmp>::new(socket, handle.sender.clone()));
                Poll::Ready(Some(Ok(())))
            }, _ => Poll::Pending
        }
    }
}

/// Quickly run the server
///
/// Submit a convenient method to quickly run Tcp and Udp instances.
pub async fn run(addrs: ServerAddr) -> Result<(), Box<dyn Error>> {
    let (sender, receiver) = mpsc::unbounded_channel();
    let forward = Forward::new(addrs.produce, receiver).await?;
    let mut server = Server::new(addrs.consume, sender).await?;
    tokio::spawn(forward);
    loop {
        server.next().await;
    }
}
