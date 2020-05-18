use crate::peer::{Tx, WRx};
use bytes::BytesMut;
use futures::prelude::*;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, Error};
use tokio::net::TcpStream;
use transport::*;

pub enum StreamType {
    Undefined,
    Publish,
    Pull,
}

pub struct Socket {
    r#type: StreamType,
    transport: Transport,
    stream: TcpStream,
    receiver: Option<WRx>,
    sender: Tx,
}

impl Socket {
    pub fn new(stream: TcpStream, sender: Tx) -> Self {
        Self {
            stream,
            sender,
            receiver: None,
            r#type: StreamType::Undefined,
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
            if let Some(result) = self.transport.decoder(chunk) {
                for message in result {
                    match self.r#type {
                        StreamType::Publish => {
                            
                        },
                        StreamType::Pull => {

                        },
                        StreamType::Undefined => {
                            match message.0 {
                                Flag::Publish => {
                                    self.r#type = StreamType::Publish;
                                    self.sender.send(message.1).unwrap();
                                },
                                Flag::Pull => {
                                    self.r#type = StreamType::Pull;
                                },
                                _ => (),
                            }
                        }
                    }
                }
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
