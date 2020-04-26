use futures::prelude::*;

use std::pin::Pin;
use std::task::{
    Poll,
    Context
};

use bytes::{ 
    Bytes, 
    BytesMut
};

use tokio::{ 
    io::Error,
    net::TcpStream,
    io::AsyncWrite
};

use crate::rtmp::Rtmp;

/// TcpSocket 会话信息.
/// 
/// 处理TCP数据并交给RTMP模块处理.
/// 并抽象成Future.
pub struct Socket {
    stream: TcpStream,
    rtmp: Rtmp
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
    pub fn new (stream: TcpStream) -> Self {
        Self {
            stream,
            rtmp: Rtmp::new()
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
    pub async fn send<'b> (&mut self, ctx: &mut Context<'b>, data: &[u8]) -> Option<()> {
        match Pin::new(&mut self.stream).poll_write(ctx, &data[..]) {
            Poll::Ready(Ok(_)) => Some(()),
            _ => None
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
    pub fn read<'b> (&mut self, ctx: &mut Context<'b>, size: usize) -> Option<Bytes> {
        let mut receiver = BytesMut::with_capacity(size);
        match self.stream.poll_peek(ctx, &mut receiver) {
            Poll::Ready(Ok(rsize)) if rsize > 0 => Some(receiver.freeze()), 
            _ => None
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
    pub fn flush<'b> (&mut self, ctx: &mut Context<'b>) -> Option<()> {
        match Pin::new(&mut self.stream).poll_flush(ctx) {
            Poll::Ready(Ok(_)) => Some(()),
            _ => None
        }
    }
}

impl Future for Socket {
    type Output = Result<(), Error>;

    fn poll (self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        match this.read(ctx, 4096) {
            None => Poll::Pending,
            Some(chunk) => {
                this.rtmp
                    .process(chunk)
                    .iter()
                    .enumerate()
                    .for_each(|(_, v)| { 
                        this.send(ctx, v); 
                    });
                this.flush(ctx);
                Poll::Ready(Ok(()))
            }
        }
    }
}
