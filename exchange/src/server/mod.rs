mod socket;

use crate::router::Router;

use socket::Socket;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

/// 快速运行服务
///
/// 提供简单方便的服务器启动入口.
pub async fn run(addr: SocketAddr) -> Result<(), io::Error> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut listener = TcpListener::bind(addr).await?;

    let router = tokio::spawn(Router::new(receiver));

    while let Ok((socket, addr)) = listener.accept().await {
        let addr_str = Arc::new(addr.to_string());
        tokio::spawn(Socket::new(socket, addr_str, sender.clone()));
    }

    // 这里还有一份已经不需要的 Sender，要先于 Receiver drop掉才不会阻塞 Receiver 关闭
    drop(sender);
    router.await?
}
