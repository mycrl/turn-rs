pub mod buffer;

use futures::prelude::*;
use std::task::{Context, Poll};
use std::{marker::Unpin, pin::Pin};
use std::collections::HashMap;
use bytes::{BytesMut, BufMut};
use tokio::sync::mpsc;
use tokio::io::Error;
use buffer::Buffer;

/// Byte stream read and write pipeline type.
pub type Tx = mpsc::UnboundedSender<(String, BytesMut)>;
pub type Rx = mpsc::UnboundedReceiver<(String, BytesMut)>;

pub struct Peer {
    buffer: Buffer,
    local_receiver: Rx,
    pub receiver: Rx,
    local_sender: Tx,
    pub sender: Tx,
}

impl Peer {
    pub fn new() -> Self {
        let (sender, local_receiver) = mpsc::unbounded_channel();
        let (local_sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            local_sender,
            local_receiver,
            buffer: Buffer::new(),
        }
    }

    fn process<'b>(&mut self, ctx: &mut Context<'b>) {
        while let Poll::Ready(Some((channel, data))) = Pin::new(&mut self.local_receiver).poll_next(ctx) {
            
        }
    }
}

impl Future for Peer {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
