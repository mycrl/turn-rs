use super::Rx;
use futures::prelude::*;
use std::io::Error;
use std::net::{SocketAddr, UdpSocket};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Udp instance
///
/// Responsible for broadcasting audio and video data 
/// and control information. Global Rtmp audio and video 
/// messages are sent to the far end through this module.
pub struct Dgram {
    addr: SocketAddr,
    dgram: UdpSocket,
    receiver: Rx,
}

impl Dgram {
    /// Create Udp instance
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use dgram::Dgram;
    /// use tokio::sync::mpsc;
    ///
    /// let addr = "0.0.0.0:1936".parse().unwrap();
    /// let (_, receiver) = mpsc::unbounded_channel();
    ///
    /// Dgram::new(addr, receiver).await.unwrap();
    /// ```
    pub fn new(addr: SocketAddr, receiver: Rx) -> Result<Self, Error> {
        Ok(Self {
            addr,
            receiver,
            dgram: UdpSocket::bind("0.0.0.0:0")?,
        })
    }

    /// Send data to remote Udp server
    ///
    /// Write Udp data to UdpSocket.
    /// Check whether the writing is completed, 
    /// if not completely written, write the rest again.
    ///
    /// TODO: 异常处理未完善, 未处理意外情况，可能会出现死循环;
    #[rustfmt::skip]
    fn send(&mut self, data: &[u8]) {
        let mut offset: usize = 0;
        let length = data.len();
        loop {
            match self.dgram.send_to(data, self.addr) {
                Ok(s) => match &offset + &s >= length {
                    false => { offset += s; }
                    true => { break; }
                }, _ => ()
            }
        }
    }

    /// Try to process Udp packets in the pipeline
    ///
    /// Repeated attempts to read pipeline data,
    /// If read, send data packet and continue to try.
    fn process<'b>(&mut self, ctx: &mut Context<'b>) {
        while let Poll::Ready(Some(data)) = Pin::new(&mut self.receiver).poll_next(ctx) {
            self.send(&data);
        }
    }
}

impl Future for Dgram {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
