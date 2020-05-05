use crate::codec::rtmp::Rtmp;
use futures::prelude::*;
use std::pin::Pin;
use std::task::{Poll, Context};
use bytes::{Bytes, BytesMut};
use tokio::{io::Error, net::TcpStream};
use tokio::{io::AsyncWrite, io::AsyncRead};

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
    pub fn new (stream: TcpStream) -> Self {
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
    pub fn send<'b> (&mut self, ctx: &mut Context<'b>, data: &[u8]) {
        let mut offset: usize = 0;
        loop {
            match Pin::new(&mut self.stream).poll_write(ctx, &data[..]) {
                Poll::Ready(Ok(size)) => match &data.len() < &offset {
                    true => { offset += size; },
                    false => { break; }
                }, _ => ()
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
    pub fn read<'b> (&mut self, ctx: &mut Context<'b>) -> Option<Bytes> {
        let mut receiver = [0u8; 2048];
        match Pin::new(&mut self.stream).poll_read(ctx, &mut receiver) {
            Poll::Ready(Ok(rsize)) if rsize > 0 => Some(BytesMut::from(&receiver[0..rsize]).freeze()), 
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
    #[rustfmt::skip]
    pub fn flush<'b> (&mut self, ctx: &mut Context<'b>) {
        loop {
            match Pin::new(&mut self.stream).poll_flush(ctx) {
                Poll::Ready(Ok(_)) => { break; },
                _ => (),
            }
        }
    }

    /// 尝试处理TcpSocket数据.
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
    ///             socket.process(cx);
    ///         });
    ///     }
    /// }
    /// ```
    pub fn process<'b> (&mut self, ctx: &mut Context<'b>) {
        let mut handle = Pin::new(self);
        if let Some(chunk) = handle.read(ctx) {
            if let Some(buffer) = handle.codec.process(chunk) {
                handle.send(ctx, &buffer[..]);
                handle.flush(ctx);
            }
        }
    }
}

impl Future for Socket {
    type Output = Result<(), Error>;

    #[rustfmt::skip]
    fn poll (self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
