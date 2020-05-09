pub mod socket;

use crate::codec::rtmp::Rtmp;
use futures::prelude::*;
use socket::Socket;
use std::pin::Pin;
use std::error::Error;
use std::net::SocketAddr;
use std::task::{Context, Poll};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::UnboundedReceiver;
use bytes::Bytes;

pub struct ServerAddress {
    pub tcp: SocketAddr,
    pub udp: SocketAddr
}

/// TCP 服务器.
///
/// 创建一个TCP服务器，绑定到指定端口地址并处理RTMP协议消息.
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
    udp: UdpSocket,
    sender: UnboundedSender<Bytes>,
    receiver: UnboundedReceiver<Bytes>
}

impl Server {
    /// 创建TCP服务器.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::Server;
    ///
    /// let addr = "0.0.0.0:1935".parse().unwrap();
    /// Server::new(addr).await.unwrap();
    /// ```
    pub async fn new(addrs: ServerAddress) -> Result<Self, Box<dyn Error>> {
        let (sender, receiver) = unbounded_channel();
        Ok(Self {
            sender, 
            receiver,
            tcp: TcpListener::bind(&addrs.tcp).await?,
            udp: UdpSocket::bind(&addrs.udp).await?
        })
    }

    pub async fn send(&mut self, data: &[u8]) {
        let mut offset: usize = 0;
        loop {
            match self.udp.send(data).await {
                Ok(size) => {
                    offset += size;
                    if &offset >= &data.len() { break; }
                }, _ => (),
            }
        }
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
            }, _ => ()
        }

        match handle.receiver.try_recv() {
            Ok(data) => {
                println!("send udp data {:?}", &data.len());
                handle.send(&data);
                Poll::Ready(Some(Ok(())))
            }, _ => Poll::Pending
        }
    }
}
