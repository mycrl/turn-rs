pub mod socket;

use futures::prelude::*;
use socket::Socket;
use std::error::Error;
use std::net::SocketAddr;
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
    pub fn new(addr: SocketAddr) -> Result<Self, Box<dyn Error>> {
        Ok(Self(TcpListener::bind(&addr)?))
    }
}

impl Future for Server {
    type Item = ();
    type Error = ();

    #[rustfmt::skip]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Ok(Async::Ready((socket, _))) = self.0.poll_accept() {
            tokio::spawn(Socket::new(socket));
        }

        Ok(Async::NotReady)
    }
}
