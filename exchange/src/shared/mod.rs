pub mod peer;

use futures::prelude::*;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::collections::HashMap;
use tokio::sync::watch;
use tokio::sync::mpsc;
use tokio::io::Error;
use bytes::BytesMut;

/// Byte stream read and write pipeline type.
pub type Tx = mpsc::UnboundedSender<(String, BytesMut)>;
pub type Rx = mpsc::UnboundedReceiver<(String, BytesMut)>;
pub type Wrx = watch::Receiver<BytesMut>;
pub type Wtx = watch::Sender<BytesMut>;

pub struct Shared {
    buffer: HashMap<String, Vec<BytesMut>>,
    channel: HashMap<String, Wtx>,
    receiver: Rx,
    pub sender: Tx,
}

impl Shared {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            buffer: HashMap::new(),
            channel: HashMap::new(),
        }
    }

    fn push(&mut self, channel: String) {
        if self.buffer.contains_key(&channel) == false {
            self.buffer.insert(channel, vec![]);
            // self.channel.insert(channel, )
        }
    }

    fn process<'b>(&mut self, ctx: &mut Context<'b>) {
        while let Poll::Ready(Some((channel, data))) = Pin::new(&mut self.receiver).poll_next(ctx) {
            let queue = self.buffer.entry(channel).or_insert(vec![]);
            queue.push(data)
        }
    }
}

impl Future for Shared {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
