use tokio::sync::RwLock;
use bytes::Bytes;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
};

use tokio::sync::mpsc::{
    channel,
    Receiver,
    Sender,
};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Transport {
    TCP,
    UDP,
}

#[derive(Default)]
pub struct Router {
    senders:
        RwLock<HashMap<(Transport, SocketAddr), Sender<(Bytes, SocketAddr)>>>,
    map: RwLock<HashMap<SocketAddr, (Transport, SocketAddr)>>,
}

impl Router {
    pub async fn find(&self, addr: &SocketAddr) -> bool {
        self.map.read().await.get(addr).is_some()
    }

    pub async fn register(
        &self,
        transport: Transport,
        interface: SocketAddr,
        addr: SocketAddr,
    ) {
        self.map.write().await.insert(addr, (transport, interface));
    }

    pub async fn get_receiver(
        self: &Arc<Self>,
        transport: Transport,
        addr: SocketAddr,
    ) -> Receiver<(Bytes, SocketAddr)> {
        let (sender, mut receiver) = channel(10);

        {
            self.senders.write().await.insert((transport, addr), sender);
        }

        let this = self.clone();
        let (writer, reader) = channel(10);
        tokio::spawn(async move {
            while let Some(bytes) = receiver.recv().await {
                if writer.send(bytes).await.is_err() {
                    this.remove_sender(&(transport, addr)).await;
                    break;
                }
            }
        });

        reader
    }

    pub async fn send(&self, addr: &SocketAddr, data: &[u8]) {
        let mut destroy = None;

        {
            if let Some(node) = self.map.read().await.get(addr) {
                if let Some(sender) = self.senders.read().await.get(node) {
                    if sender
                        .send((Bytes::copy_from_slice(data), addr.clone()))
                        .await
                        .is_err()
                    {
                        destroy = Some(*node);
                    }
                }
            }
        }

        if let Some(node) = destroy {
            self.remove_sender(&node).await;
        }
    }

    async fn remove_sender(&self, node: &(Transport, SocketAddr)) {
        if let Some(sender) =
            self.senders.write().await.remove(node)
        {
            drop(sender)
        }
    }

    async fn remove(&self, addr: &SocketAddr) {
        self.map.write().await.remove(addr);
    }
}
