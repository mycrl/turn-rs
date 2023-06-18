use bitvec::prelude::*;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
};

use tokio::sync::{
    RwLock,
    Mutex,
};

use tokio::sync::mpsc::{
    unbounded_channel,
    UnboundedReceiver,
    UnboundedSender,
};

pub struct Router {
    senders: RwLock<HashMap<u8, UnboundedSender<(Vec<u8>, SocketAddr)>>>,
    bits: Mutex<&'static mut BitSlice<u8, Lsb0>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            senders: Default::default(),
            bits: Mutex::new(unsafe { bits![static mut u8, Lsb0; 1; 255] }),
        }
    }

    pub async fn get_receiver(
        self: &Arc<Self>,
    ) -> (u8, UnboundedReceiver<(Vec<u8>, SocketAddr)>) {
        let index = self
            .alloc_index()
            .await
            .expect("transport router alloc index failed!");
        let (sender, receiver) = unbounded_channel();
        self.senders.write().await.insert(index, sender);
        (index, receiver)
    }

    pub async fn send(&self, index: u8, addr: &SocketAddr, data: &[u8]) {
        let mut is_destroy = false;

        {
            if let Some(sender) = self.senders.read().await.get(&index) {
                if sender.send((data.to_vec(), addr.clone())).is_err() {
                    is_destroy = true;
                }
            }
        }

        if is_destroy {
            self.remove(index).await;
        }
    }

    pub async fn remove(&self, index: u8) {
        if let Some(sender) = self.senders.write().await.remove(&index) {
            self.free_index(index).await;
            drop(sender)
        }
    }

    async fn alloc_index(&self) -> Option<u8> {
        let mut bits = self.bits.lock().await;
        let index = bits.first_one().map(|i| i as u8)?;
        bits.set(index as usize, false);
        Some(index)
    }

    async fn free_index(&self, index: u8) {
        self.bits.lock().await.set(index as usize, true);
    }
}
