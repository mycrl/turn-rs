use bytes::BytesMut;
use futures::prelude::*;
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::{mpsc, watch};

/// Byte stream read and write pipeline type.
pub type Tx = mpsc::UnboundedSender<BytesMut>;
pub type Rx = mpsc::UnboundedReceiver<BytesMut>;
pub type WRx = watch::Receiver<BytesMut>;
pub type WTx = watch::Sender<BytesMut>;

pub struct Peer {
    channel: HashMap<String, (WTx, WRx)>,
    metadata: HashMap<String, BytesMut>,
    receiver: Rx,
    pub sender: Tx,
}

impl Peer {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            channel: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn get_channel(&mut self, channel: String) -> Option<WRx> {
        match self.channel.get(&channel) {
            Some((_, receiver)) => Some(receiver.clone()),
            None => None,
        }
    }

    pub fn set_metadata(&mut self, channel: String, data: BytesMut) {
        self.metadata.insert(channel, data);
    }

    fn push(&mut self, channel: String, data: BytesMut) {
        if self.channel.contains_key(&channel) == false {
            self.channel.insert(channel, watch::channel(data));
        } else if let Some((sender, _)) = self.channel.get(&channel) {
            sender.broadcast(data).unwrap();
        }
    }

    fn decoder(&mut self, data: BytesMut) {
        
    }

    fn process<'b>(&mut self, ctx: &mut Context<'b>) {
        while let Poll::Ready(Some(data)) = Pin::new(&mut self.receiver).poll_next(ctx) {
            
        }
    }
}

impl Future for Peer {
    type Output = Result<(), tokio::io::Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
