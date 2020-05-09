use crate::codec::{Codec, Packet};
use bytes::{Bytes, BytesMut};
use futures::prelude::*;
use std::pin::Pin;
use std::marker::Unpin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, Error};
use tokio::sync::mpsc::UnboundedSender;
use tokio::net::TcpStream;

pub struct Socket<T> {
    dgram: UnboundedSender<Bytes>,
    stream: TcpStream,
    codec: T,
}

impl <T: Default + Codec + Unpin>Socket<T> {
    pub fn new(stream: TcpStream, dgram: UnboundedSender<Bytes>) -> Self {
        Self {
            dgram,
            stream,
            codec: T::default(),
        }
    }

    pub fn push(&mut self, data: Bytes) {
        loop {
            match self.dgram.send(data.clone()) {
                Ok(_) => { break; },
                _ => (),
            }
        }
    }

    /// 发送数据到 TcpSocket.
    #[rustfmt::skip]
    pub fn send<'b>(&mut self, ctx: &mut Context<'b>, data: &[u8]) {
        let mut offset: usize = 0;
        loop {
            match Pin::new(&mut self.stream).poll_write(ctx, &data) {
                Poll::Ready(Ok(size)) => {
                    offset += size;
                    if &offset >= &data.len() { break; }
                }, _ => (),
            }
        }
    }

    /// 从 TcpSocket 读取数据.
    #[rustfmt::skip]
    pub fn read<'b>(&mut self, ctx: &mut Context<'b>) -> Option<Bytes> {
        let mut receiver = [0u8; 2048];
        match Pin::new(&mut self.stream).poll_read(ctx, &mut receiver) {
            Poll::Ready(Ok(s)) if s > 0 => Some(BytesMut::from(&receiver[0..s]).freeze()), 
            _ => None
        }
    }

    /// 刷新 TcpSocket 缓冲区.
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
    pub fn process<'b>(&mut self, ctx: &mut Context<'b>) {
        while let Some(chunk) = self.read(ctx) {
            for packet in self.codec.parse(chunk) {
                match packet {
                    Packet::Tcp(data) => self.send(ctx, &data),
                    Packet::Udp(data) => self.push(data),
                }
            }

            self.flush(ctx);
        }
    }
}

impl <T: Default + Codec + Unpin>Future for Socket<T> {
    type Output = Result<(), Error>;

    #[rustfmt::skip]
    fn poll (self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
