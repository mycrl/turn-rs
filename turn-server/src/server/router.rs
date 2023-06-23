use bitvec::prelude::*;
use turn_rs::StunClass;
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

/// Handles packet forwarding between transport protocols.
pub struct Router {
    senders:
        RwLock<HashMap<u8, UnboundedSender<(Vec<u8>, StunClass, SocketAddr)>>>,
    bits: Mutex<&'static mut BitSlice<u8, Lsb0>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            senders: Default::default(),
            bits: Mutex::new(unsafe { bits![static mut u8, Lsb0; 1; 255] }),
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

    /// Get the endpoint reader for the route.
    ///
    /// Each transport protocol is layered according to its own endpoint, and
    /// the data forwarded to this endpoint can be obtained by routing.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let router = Router::new()
    /// let (index, receiver) = router.get_receiver().await;
    ///
    /// while let Some(res) = receiver.recv().await {
    ///     // handle res.
    /// }
    /// ```
    pub async fn get_receiver(
        self: &Arc<Self>,
    ) -> (u8, UnboundedReceiver<(Vec<u8>, StunClass, SocketAddr)>) {
        let index = self
            .alloc_index()
            .await
            .expect("transport router alloc index failed!");
        let (sender, receiver) = unbounded_channel();
        self.senders.write().await.insert(index, sender);
        (index, receiver)
    }

    /// Send data to router.
    ///
    /// By specifying the endpoint identifier and destination address, the route
    /// is forwarded to the corresponding endpoint. However, it should be noted
    /// that calling this function will not notify whether the endpoint exists.
    /// If it does not exist, the data will be discarded by default.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let router = Router::new()
    /// let (index, receiver) = router.get_receiver().await;
    ///
    /// router.send(index, "127.0.0.1:8080".parse().unwrap(), b"hello").await;
    ///
    /// while let Some((data, target)) = receiver.recv().await {
    ///     println!("{}, {:?}", data.len(), target);
    /// }
    /// ```
    pub async fn send(
        &self,
        index: u8,
        class: StunClass,
        addr: &SocketAddr,
        data: &[u8],
    ) {
        let mut is_destroy = false;

        {
            if let Some(sender) = self.senders.read().await.get(&index) {
                if sender.send((data.to_vec(), class, addr.clone())).is_err() {
                    is_destroy = true;
                }
            }
        }

        if is_destroy {
            self.remove(index).await;
        }
    }

    /// delete endpoint.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let router = Router::new()
    /// let (index, receiver) = router.get_receiver().await;
    ///
    /// router.remove(index).await;
    /// router.send(index, "127.0.0.1:8080".parse().unwrap(), b"hello").await;
    ///
    /// while let Some((data, target)) = receiver.recv().await {
    ///     println!("{}, {:?}", data.len(), target);
    /// }
    /// ```
    pub async fn remove(&self, index: u8) {
        if let Some(sender) = self.senders.write().await.remove(&index) {
            self.free_index(index).await;
            drop(sender)
        }
    }
}
