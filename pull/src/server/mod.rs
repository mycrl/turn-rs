mod porter;
mod socket;

use futures::prelude::*;
use porter::Porter;
use socket::Socket;
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{io::Error, pin::Pin};
use tokio::sync::mpsc;
use transport::{Flag, Payload};

/// 事件传递通道
pub type Rx = mpsc::UnboundedReceiver<Event>;
pub type Tx = mpsc::UnboundedSender<Event>;

/// 事件
pub enum Event {
    Subscribe(String, Tx),
    Bytes(Flag, Arc<Payload>),
}

/// 服务器地址
pub struct ServerAddr {
    pub consume: SocketAddr,
    pub produce: SocketAddr,
}

/// Tcp服务器
///
/// 主要处理WebSocket连接,
/// 对其他类型的不处理.
pub struct Server {
    listener: TcpListener,
    sender: Tx,
}

impl Server {
    /// 创建WebSocket服务器实例
    pub fn new(addr: SocketAddr, sender: Tx) -> Result<Self, Error> {
        Ok(Self {
            sender,
            listener: TcpListener::bind(addr)?,
        })
    }
}

impl Stream for Server {
    type Item = Result<(), Box<dyn std::error::Error>>;

    #[rustfmt::skip]
    fn poll_next (self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
        let handle = self.get_mut();
        match handle.listener.accept() {
            Ok((socket, _)) => {
                tokio::spawn(Socket::new(socket, handle.sender.clone())?);
                Poll::Ready(Some(Ok(())))
            }, _ => Poll::Pending
        }
    }
}

/// 快速运行服务
///
/// 提供简单方便的服务器启动入口.
pub async fn run(addrs: ServerAddr) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, receiver) = mpsc::unbounded_channel();
    let poter = Porter::new(addrs.produce, receiver).await?;
    let mut server = Server::new(addrs.consume, sender)?;
    tokio::spawn(poter);
    loop {
        server.next().await;
    }
}
