use futures::prelude::*;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, Error};
use tokio::net::TcpStream;
use transport::Transport;
use bytes::BytesMut;

pub struct Socket {
    stream: TcpStream,
    transport: Transport,
}

impl Socket {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            transport: Transport::new(),
        }
    }

    /// Send data to TcpSocket
    ///
    /// Write Tcp data to TcpSocket.
    /// Check whether the writing is completed, 
    // if not completely written, write the rest.
    ///
    /// TODO: 异常处理未完善, 未处理意外情况，可能会出现死循环;
    #[rustfmt::skip]
    #[allow(dead_code)]
    fn send<'b>(&mut self, ctx: &mut Context<'b>, data: &[u8]) {
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

    /// Read data from TcpSocket
    ///
    /// TODO: 目前存在重复申请缓冲区的情况，有优化空间；
    #[rustfmt::skip]
    #[allow(dead_code)]
    fn read<'b>(&mut self, ctx: &mut Context<'b>) -> Option<BytesMut> {
        let mut receiver = [0u8; 2048];
        match Pin::new(&mut self.stream).poll_read(ctx, &mut receiver) {
            Poll::Ready(Ok(s)) if s > 0 => Some(BytesMut::from(&receiver[0..s])),
            _ => None,
        }
    }

    /// Refresh the TcpSocket buffer
    ///
    /// After writing data to TcpSocket, you need to refresh 
    /// the buffer and send the data to the peer.
    ///
    /// TODO: 异常处理未完善, 未处理意外情况，可能会出现死循环;
    #[rustfmt::skip]
    #[allow(dead_code)]
    fn flush<'b>(&mut self, ctx: &mut Context<'b>) {
        loop {
            match Pin::new(&mut self.stream).poll_flush(ctx) {
                Poll::Ready(Ok(_)) => { break; },
                _ => (),
            }
        }
    }

    /// Try to process TcpSocket data
    ///
    /// Use transport module decoder to handle TcpSocket data,
    /// Write the returned data to TcpSocket or UdpSocket correctly.
    #[rustfmt::skip]
    fn process<'b>(&mut self, ctx: &mut Context<'b>) {
        while let Some(chunk) = self.read(ctx) {
            if let Some(results) = self.transport.decoder(chunk) {

            }
        }
    }
}

impl Future for Socket {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
