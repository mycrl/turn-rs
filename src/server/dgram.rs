use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::UnboundedReceiver;
use std::net::UdpSocket;
use std::net::SocketAddr;
use tokio::io::Error;
use futures::prelude::*;
use bytes::Bytes;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Dgram {
    dgram: UdpSocket,
    sender: UnboundedSender<Bytes>,
    receiver: UnboundedReceiver<Bytes>
}

impl Dgram {
    pub fn new (addr: &SocketAddr) -> Result<Self, Error> {
        let (sender, receiver) = unbounded_channel();
        Ok(Self {
            sender, 
            receiver,
            dgram: UdpSocket::bind(addr)?
        })
    }

    pub fn get_sender (&mut self) -> UnboundedSender<Bytes> {
        self.sender.clone()
    }

    pub fn send(&mut self, data: &[u8]) {
        let mut offset: usize = 0;
        loop {
            match self.dgram.send(data) {
                Ok(size) => {
                    offset += size;
                    if &offset >= &data.len() { break; }
                }, _ => (),
            }
        }
    }

    pub fn process (&mut self) {
        match self.receiver.try_recv() {
            Ok(data) => {
                println!("udp data");
                self.send(&data);
            },
            _ => ()
        }
    }
}

impl Future for Dgram {
    type Output = Result<(), Error>;

    #[rustfmt::skip]
    fn poll(self: Pin<&mut Self>, _: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process();
        Poll::Pending
    }
}