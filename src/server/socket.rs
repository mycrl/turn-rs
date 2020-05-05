use crate::rtmp::Rtmp;
use futures::prelude::*;
use bytes::{Bytes, BytesMut};
use tokio::{net::TcpStream, io::AsyncRead, io::AsyncWrite};
use std::io::Write;

enum State {
    Data(Bytes),
    NotData,
    Close
}

/// TcpSocket 会话信息.
///
/// 处理TCP数据并交给RTMP模块处理.
/// 并抽象成Future.
pub struct Socket {
    stream: TcpStream,
    codec: Rtmp
}

impl Socket {
    /// 创建 TcpSocket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    /// use tokio::net::TcpListener;
    /// use socket::Socket;
    ///
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let addr = "0.0.0.0:1935".parse().unwrap();
    ///     let mut listener = TcpListener::bind(&addr).await?;
    ///     
    ///     loop {
    ///         let (stream, _) = listener.accept().await?;
    ///         tokio::spawn(Socket::new(stream));
    ///     }
    /// }
    /// ```
    pub fn new(stream: TcpStream) -> Self {
        Self { 
            stream,
            codec: Rtmp::new()
        }
    }

    /// 发送数据到 TcpSocket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    /// use futures::future::poll_fn;
    /// use tokio::net::TcpListener;
    /// use socket::Socket;
    ///
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let addr = "0.0.0.0:1935".parse().unwrap();
    ///     let mut listener = TcpListener::bind(&addr).await?;
    ///     
    ///     loop {
    ///         let (stream, _) = listener.accept().await?;
    ///         let socket = Socket::new(stream);
    ///
    ///         poll_fn(|cx| {
    ///             socket.send(cx, &[0, 1, 2]);
    ///         });
    ///     }
    /// }
    /// ```
    #[rustfmt::skip]
    pub fn send(&mut self, data: &[u8]) {
        loop {
            match self.stream.write_all(data) {
                Ok(_) => { break; },
                _ => (),
            }
        }
    }

    /// 从 TcpSocket 读取数据.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    /// use futures::future::poll_fn;
    /// use tokio::net::TcpListener;
    /// use socket::Socket;
    ///
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let addr = "0.0.0.0:1935".parse().unwrap();
    ///     let mut listener = TcpListener::bind(&addr).await?;
    ///     
    ///     loop {
    ///         let (stream, _) = listener.accept().await?;
    ///         let socket = Socket::new(stream);
    ///
    ///         poll_fn(|cx| {
    ///             socket.read(cx, 128);
    ///         });
    ///     }
    /// }
    /// ```
    #[rustfmt::skip]
    fn read(&mut self) -> State {
        let mut receiver = [0; 4096];
        match self.stream.poll_read(&mut receiver) {
            Ok(Async::Ready(size)) if size > 0 => State::Data(BytesMut::from(&receiver[0..size]).freeze()),
            Ok(Async::Ready(size)) if size == 0 => State::Close,
            _ => State::NotData,
        }
    }

    /// 刷新 TcpSocket 缓冲区.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    /// use futures::future::poll_fn;
    /// use tokio::net::TcpListener;
    /// use socket::Socket;
    ///
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let addr = "0.0.0.0:1935".parse().unwrap();
    ///     let mut listener = TcpListener::bind(&addr).await?;
    ///     
    ///     loop {
    ///         let (stream, _) = listener.accept().await?;
    ///         let socket = Socket::new(stream);
    ///
    ///         poll_fn(|cx| {
    ///             socket.read(cx, 128);
    ///             socket.flush(cx);
    ///         });
    ///     }
    /// }
    /// ```
    pub fn flush(&mut self) {
        loop {
            match self.stream.poll_flush() {
                Ok(Async::Ready(_)) => { break; },
                _ => (),
            }
        }
    }
}


impl Future for Socket {
    type Item = ();
    type Error = ();

    #[rustfmt::skip]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let State::Data(buffer) = self.read() {
            let receiver = self.codec.process(buffer);
            self.send(&receiver[..]);
            self.flush();
        }

        Ok(Async::NotReady)
    }
}
