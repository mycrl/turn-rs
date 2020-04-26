pub mod socket;

use futures::prelude::*;
use std::io::Error;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use socket::Socket;
use tokio::net::TcpListener;

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
pub struct Server(TcpListener);

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
    pub async fn new(addr: SocketAddr) -> Result<Self, Box<Error>> {
        Ok(Self(TcpListener::bind(&addr).await?))
    }
}

impl Stream for Server {
    type Item = Result<(), Error>;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            Self(listener) => match listener.poll_accept(ctx) {
                Poll::Ready(Ok((socket, _))) => {
                    tokio::spawn(Socket::new(socket));
                    Poll::Ready(Some(Ok(())))
                }
                _ => Poll::Pending,
            },
        }
    }
}
