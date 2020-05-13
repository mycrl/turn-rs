pub mod socket;
pub mod transport;
pub mod dgram;

use crate::codec::rtmp::Rtmp;
use futures::prelude::*;
use socket::Socket;
use std::pin::Pin;
use std::error::Error;
use std::net::SocketAddr;
use std::task::{Context, Poll};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use bytes::Bytes;
use dgram::Dgram;

/// 字节流读写管道类型.
pub type Tx = mpsc::UnboundedSender<Bytes>;
pub type Rx = mpsc::UnboundedReceiver<Bytes>;

/// 服务器地址.
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
    sender: Tx
}

impl Server {
    /// 创建TCP服务器.
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

/// 快速运行服务器
/// 
/// 提交便捷方法，快速运行Tcp和Udp实例.
pub async fn run(addrs: ServerAddress) -> Result<(), Box<dyn Error>> {
    let (sender, receiver) = mpsc::unbounded_channel();
    let mut server = Server::new(addrs.tcp, sender).await?;
    tokio::spawn(Dgram::new(addrs.udp, receiver)?);
    loop { server.next().await; }
}
