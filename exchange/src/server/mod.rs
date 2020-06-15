mod socket;

use crate::router::Router;
use crate::router::{Rx, Tx};
use std::{io::Error, sync::Arc};
use tokio::net::TcpListener;
use configure::ConfigureModel;
use socket::Socket;

/// 运行路由服务
///
/// 路由中心提供频道数据的路由和转发.
#[allow(warnings)]
fn run_router(receiver: Rx) {
    let mut router = Router::new(receiver);
    tokio::spawn(async move {
        loop { router.process().await;}
        Ok::<(), Error>(())
    });
}

/// 运行TCP服务器
/// 
/// 维护和处理TCP Socket链接与数据.
#[allow(warnings)]
async fn run_server(mut listener: TcpListener, sender: Tx) {
    while let Ok((stream, addr)) = listener.accept().await {
        let addr_str = Arc::new(addr.to_string());
        let mut socket = Socket::new(stream, addr_str, sender.clone());
        tokio::spawn(async move {
            loop { socket.process().await?;}
            Ok::<(), Error>(())
        });
    }
}

/// 快速运行服务
///
/// 提供简单方便的服务器启动入口.
#[rustfmt::skip]
pub async fn run(configure: ConfigureModel) -> Result<(), Error> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let listener = TcpListener::bind(configure.exchange.to_addr()).await?;
    run_router(receiver);
    run_server(listener, sender).await;
    Ok(())
}
