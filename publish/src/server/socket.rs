use super::Tx;
use codec::{Codec, Packet};
use bytes::BytesMut;
use futures::prelude::*;
use std::task::{Context, Poll};
use std::{marker::Unpin, pin::Pin};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::{Error, ErrorKind};
use tokio::net::TcpStream;
use transport::Flag;

/// TcpSocket instance
///
/// Read and write TcpSocket and return data through channel.
/// The returned data is a Udp data packet. In order to adapt to MTU,
/// the subcontracting has been completed.
pub struct Socket<T> {
    stream: TcpStream,
    forward: Tx,
    codec: T,
}

impl<T: Default + Codec + Unpin> Socket<T> {
    /// Create a TcpSocket instance
    ///
    /// To create an instance, you need to specify a `Codec` as the data codec.
    /// `Codec` processes Tcp data and asks for the returned Tcp data and Udp packet.
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
    pub fn new(stream: TcpStream, forward: Tx) -> Self {
        Self {
            stream,
            forward,
            codec: T::default(),
        }
    }

    /// Push messages to channel
    ///
    /// Push the chunk package to the channel.
    /// The other end needs to send data to the remote TcpServer.
    #[rustfmt::skip]
    fn push(&mut self, data: BytesMut, flag: Flag) -> Result<(), Error> {
        match self.forward.send((flag, data)) {
            Err(e) => Err(Error::new(ErrorKind::BrokenPipe, e.to_string())),
            Ok(_) => Ok(())
        }
    }

    /// Send data to TcpSocket
    ///
    /// Write Tcp data to TcpSocket.
    /// Check whether the writing is completed,
    // if not completely written, write the rest.
    #[rustfmt::skip]
    fn send<'b>(&mut self, ctx: &mut Context<'b>, data: &[u8]) -> Result<(), Error> {
        let mut offset: usize = 0;
        let length = data.len();
        loop {
            match Pin::new(&mut self.stream).poll_write(ctx, &data) {
                Poll::Ready(Err(e)) => { return Err(Error::new(ErrorKind::NotConnected, e)); }, 
                Poll::Ready(Ok(s)) => match offset + s >= length {
                    false => { offset += s; },
                    true => { break; }
                }, _ => (),
            }
        }

        Ok(())
    }

    /// Read data from TcpSocket
    ///
    /// TODO: 目前存在重复申请缓冲区的情况，有优化空间；
    #[rustfmt::skip]
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
    #[rustfmt::skip]
    fn flush<'b>(&mut self, ctx: &mut Context<'b>) -> Result<(), Error> {
        loop {
            match Pin::new(&mut self.stream).poll_flush(ctx) {
                Poll::Ready(Err(e)) => { return Err(Error::new(ErrorKind::NotConnected, e)); },
                Poll::Ready(Ok(_)) => { break; },
                _ => (),
            }
        }

        Ok(())
    }

    /// Try to process TcpSocket data
    ///
    /// Use `Codec` to handle TcpSocket data,
    /// Write the returned data to TcpSocket or UdpSocket correctly.
    fn process<'b>(&mut self, ctx: &mut Context<'b>) -> Result<(), Error> {
        while let Some(mut chunk) = self.read(ctx) {
            for packet in self.codec.parse(&mut chunk) {
                match packet {
                    Packet::Peer(data) => self.send(ctx, &data)?,
                    Packet::Core(data, flag) => self.push(data, flag)?,
                }
            }

            // Refresh the TcpSocket buffer. In order to increase efficiency,
            // all the returned data of the current task will be written and
            // then refreshed in a unified manner to avoid unnecessary frequent operations.
            self.flush(ctx)?;
        }

        Ok(())
    }
}

impl<T: Default + Codec + Unpin> Future for Socket<T> {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match self.get_mut().process(ctx) {
            Ok(_) => Poll::Pending,
            Err(_) => Poll::Ready(Ok(()))
        }
    }
}
