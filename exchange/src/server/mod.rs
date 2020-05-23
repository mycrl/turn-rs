mod socket;

use super::router::{Router, Tx};
use futures::prelude::*;
use socket::Socket;
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{io::Error, pin::Pin};
use tokio::net::TcpListener;

/// TCP服务器
///
/// 处理所有连接到交换中心
/// 的TcpSocket.
pub struct Server {
    listener: TcpListener,
    sender: Tx,
}

impl Server {
    /// 创建Tcp服务器实例
    ///
    /// 接受写入管道，用于和核心
    /// 路由之间通信.
    pub async fn new(addr: SocketAddr, sender: Tx) -> Result<Self, Error> {
        Ok(Self {
            sender,
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
            Poll::Ready(Ok((socket, addr))) => {
                let addr_str = Arc::new(addr.to_string());
                let sender = handle.sender.clone();
                tokio::spawn(Socket::new(socket, addr_str, sender));
                Poll::Ready(Some(Ok(())))
            }, _ => Poll::Pending
        }
    }
}

/// 快速运行服务
///
/// 提供简单方便的服务器启动入口.
pub async fn run(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut server = Server::new(addr, sender).await?;
    tokio::spawn(Router::new(receiver));
    loop {
        server.next().await;
    }
}
