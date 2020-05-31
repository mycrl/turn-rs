mod socket;

use crate::router::{Router, Tx};

use socket::Socket;
use std::net::SocketAddr;
use std::sync::Arc;

use std::io::Error;
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

    pub async fn handle(mut self) {
        while let Ok((socket, addr)) = self.listener.accept().await {
            let addr_str = Arc::new(addr.to_string());
            let sender = self.sender.clone();
            tokio::spawn(Socket::new(socket, addr_str, sender));
        }

        // 是否需要提前关闭 sender？
        // drop(self.sender);
    }
}

/// 快速运行服务
///
/// 提供简单方便的服务器启动入口.
pub async fn run(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let server = Server::new(addr, sender).await?;
    tokio::spawn(server.handle());
    tokio::spawn(Router::new(receiver));

    Ok(())
}
