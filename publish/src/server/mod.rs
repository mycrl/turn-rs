pub mod forward;
pub mod socket;

use crate::codec::Rtmp;
use bytes::BytesMut;
use forward::Forward;
use socket::Socket;
use std::io::Error;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use configure::ConfigureModel;
use transport::Flag;

/// Byte stream read and write pipeline type.
pub type Tx = mpsc::UnboundedSender<(Flag, BytesMut)>;
pub type Rx = mpsc::UnboundedReceiver<(Flag, BytesMut)>;

/// 运行推送服务
///
/// 推送服务将音视频数据推送到远端交换中心.
#[allow(warnings)]
async fn run_forward(addr: SocketAddr, receiver: Rx) -> Result<(), Error> {
    let mut forward = Forward::new(addr, receiver).await?;
    tokio::spawn(async move {
        loop { forward.process().await?;}
        Ok::<(), Error>(())
    });

    Ok(())
}

/// 运行TCP服务器
/// 
/// 维护和处理TCP Socket链接与数据.
#[allow(warnings)]
async fn run_server(mut listener: TcpListener, sender: Tx) {
    while let Ok((stream, _)) = listener.accept().await {
        let mut socket = Socket::<Rtmp>::new(stream, sender.clone());
        tokio::spawn(async move {
            loop { socket.process().await?; }
            Ok::<(), Error>(())
        });
    }
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
/// 
/// Quickly run the server
/// Submit a convenient method to quickly run Tcp and Udp instances.
#[rustfmt::skip]
pub async fn run(configure: ConfigureModel) -> Result<(), Error> {
    let (sender, receiver) = mpsc::unbounded_channel();
    let listener = TcpListener::bind(configure.publish.to_addr()).await?;
    run_forward(configure.exchange.to_addr(), receiver).await?;
    run_server(listener, sender).await;
    Ok(())
}
