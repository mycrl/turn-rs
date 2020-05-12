use super::{Tx, transport::Transport};
use crate::codec::{Codec, Packet};
use futures::prelude::*;
use std::task::{Context, Poll};
use std::{pin::Pin, marker::Unpin};
use tokio::io::{AsyncRead, AsyncWrite, Error};
use tokio::net::TcpStream;
use bytes::{Bytes, BytesMut};

/// TcpSocket实例.
/// 
/// 读取写入TcpSocket并通过channel返回数据.
/// 返回的数据为Udp数据包，为适应MTU，已完成分包.
pub struct Socket<T> {
    transport: Transport,
    stream: TcpStream,
    dgram: Tx,
    codec: T
}

impl <T: Default + Codec + Unpin>Socket<T> {
    /// 创建TcpSocket实例.
    /// 
    /// 创建实例需要指定一个`Codec`做为数据编解码器.
    /// `Codec`处理Tcp数据，并要求给出返回的Tcp数据和Udp包.
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    /// use tokio::net::TcpListener;
    /// use socket::Socket;
    /// use rtmp::Rtmp;
    ///
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let addr = "0.0.0.0:1935".parse().unwrap();
    ///     let mut listener = TcpListener::bind(&addr).await?;
    ///     
    ///     loop {
    ///         let (stream, _) = listener.accept().await?;
    ///         tokio::spawn(Socket::<Rtmp>::new(stream));
    ///     }
    /// }
    /// ```
    pub fn new(stream: TcpStream, dgram: Tx) -> Self {
        Self {
            dgram,
            stream,
            codec: T::default(),
            transport: Transport::new(1000)
        }
    }

    /// 推送消息到channel中.
    /// 
    /// 将Udp包推送到channel中.
    /// 另一端需要将数据发送到远程UdpServer.
    /// 
    /// TODO: 异常处理未完善, 未处理意外情况，可能会出现死循环;
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// use bytes::Bytes;
    /// use std::error::Error;
    /// use futures::future::poll_fn;
    /// use tokio::net::TcpListener;
    /// use socket::Socket;
    /// use rtmp::Rtmp;
    ///
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let addr = "0.0.0.0:1935".parse().unwrap();
    ///     let mut listener = TcpListener::bind(&addr).await?;
    ///
    ///     loop {
    ///         let (stream, _) = listener.accept().await?;
    ///         let socket = Socket::<Rtmp>::new(stream);
    ///
    ///         poll_fn(|cx| {
    ///             socket.push(cx, Bytes::from(b"hello"));
    ///         });
    ///     }
    /// }
    /// ```
    #[rustfmt::skip]
    pub fn push(&mut self, data: Bytes, flgs: u8) {
        for chunk in self.transport.packet(data, flgs) {
            loop {
                match self.dgram.send(chunk.clone()) {
                    Ok(_) => { break; },
                    _ => (),
                }
            }
        }
    }

    /// 发送数据到TcpSocket.
    /// 
    /// 将Tcp数据写入到TcpSocket.
    /// 检查是否写入完成，如果未完全写入，写入剩余部分.
    /// 
    /// TODO: 异常处理未完善, 未处理意外情况，可能会出现死循环;
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    /// use futures::future::poll_fn;
    /// use tokio::net::TcpListener;
    /// use socket::Socket;
    /// use rtmp::Rtmp;
    ///
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let addr = "0.0.0.0:1935".parse().unwrap();
    ///     let mut listener = TcpListener::bind(&addr).await?;
    ///
    ///     loop {
    ///         let (stream, _) = listener.accept().await?;
    ///         let socket = Socket::<Rtmp>::new(stream);
    ///
    ///         poll_fn(|cx| {
    ///             socket.send(cx, &[0, 1, 2]);
    ///         });
    ///     }
    /// }
    /// ```
    #[rustfmt::skip]
    pub fn send<'b>(&mut self, ctx: &mut Context<'b>, data: &[u8]) {
        let mut offset: usize = 0;
        let length = data.len();
        loop {
            match Pin::new(&mut self.stream).poll_write(ctx, &data) {
                Poll::Ready(Ok(s)) => match &offset + &s >= length {
                    false => { offset += s; },
                    true => { break; }
                }, _ => (),
            }
        }
    }

    /// 从TcpSocket读取数据.
    /// 
    /// TODO: 目前存在重复申请缓冲区的情况，有优化空间；
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    /// use futures::future::poll_fn;
    /// use tokio::net::TcpListener;
    /// use socket::Socket;
    /// use rtmp::Rtmp;
    ///
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let addr = "0.0.0.0:1935".parse().unwrap();
    ///     let mut listener = TcpListener::bind(&addr).await?;
    ///
    ///     loop {
    ///         let (stream, _) = listener.accept().await?;
    ///         let socket = Socket::<Rtmp>::new(stream);
    ///
    ///         poll_fn(|cx| {
    ///             socket.read(cx);
    ///         });
    ///     }
    /// }
    /// ```
    #[rustfmt::skip]
    pub fn read<'b>(&mut self, ctx: &mut Context<'b>) -> Option<Bytes> {
        let mut receiver = [0u8; 2048];
        match Pin::new(&mut self.stream).poll_read(ctx, &mut receiver) {
            Poll::Ready(Ok(s)) if s > 0 =>  Some(BytesMut::from(&receiver[0..s]).freeze()), 
            _ => None,
        }
    }

    /// 刷新TcpSocket缓冲区.
    /// 
    /// 将数据写入到TcpSocket之后，需要刷新缓冲区，
    /// 将数据发送到对端.
    /// 
    /// TODO: 异常处理未完善, 未处理意外情况，可能会出现死循环;
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    /// use futures::future::poll_fn;
    /// use tokio::net::TcpListener;
    /// use socket::Socket;
    /// use rtmp::Rtmp;
    ///
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let addr = "0.0.0.0:1935".parse().unwrap();
    ///     let mut listener = TcpListener::bind(&addr).await?;
    ///
    ///     loop {
    ///         let (stream, _) = listener.accept().await?;
    ///         let socket = Socket::<Rtmp>::new(stream);
    ///
    ///         poll_fn(|cx| {
    ///             socket.read(cx, 128);
    ///             socket.flush(cx);
    ///         });
    ///     }
    /// }
    /// ```
    #[rustfmt::skip]
    pub fn flush<'b>(&mut self, ctx: &mut Context<'b>) {
        loop {
            match Pin::new(&mut self.stream).poll_flush(ctx) {
                Poll::Ready(Ok(_)) => { break; },
                _ => (),
            }
        }
    }

    /// 尝试处理TcpSocket数据.
    /// 
    /// 使用`Codec`处理TcpSocket数据，
    /// 并将返回的数据正确写入到TcpSocket或者UdpSocket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    /// use futures::future::poll_fn;
    /// use tokio::net::TcpListener;
    /// use socket::Socket;
    /// use rtmp::Rtmp;
    ///
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let addr = "0.0.0.0:1935".parse().unwrap();
    ///     let mut listener = TcpListener::bind(&addr).await?;
    ///     
    ///     loop {
    ///         let (stream, _) = listener.accept().await?;
    ///         let socket = Socket::<Rtmp>::new(stream);
    ///
    ///         poll_fn(|cx| {
    ///             socket.process(cx);
    ///         });
    ///     }
    /// }
    /// ```
    pub fn process<'b>(&mut self, ctx: &mut Context<'b>) {
        while let Some(chunk) = self.read(ctx) {
            for packet in self.codec.parse(chunk) {
                match packet {
                    Packet::Tcp(data) => self.send(ctx, &data),
                    Packet::Udp(data, flgs) => self.push(data, flgs),
                }
            }

            // 刷新TcpSocket缓冲区.
            // 为了增加效率，将在把当前任务的所有返回数据全部
            // 写入完成之后再统一刷新，避免不必要的频繁操作.
            self.flush(ctx);
        }
    }
}

impl <T: Default + Codec + Unpin>Future for Socket<T> {
    type Output = Result<(), Error>;
    fn poll (self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
