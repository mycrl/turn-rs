use std::net::SocketAddr;

use ahash::AHashMap;
use bitvec::prelude::*;
use parking_lot::{Mutex, RwLock};
use turn_rs::StunClass;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

type Receiver = UnboundedSender<(Vec<u8>, StunClass, SocketAddr)>;

/// Handles packet forwarding between transport protocols.
pub struct Router {
    senders: RwLock<AHashMap<u8, Receiver>>,
    bits: Mutex<&'static mut BitSlice<u8, Lsb0>>,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    pub fn new() -> Self {
        Self {
            senders: Default::default(),
            bits: Mutex::new(unsafe { bits![static mut u8, Lsb0; 1; 255] }),
        }
    }

    /// Get the endpoint reader for the route.
    ///
    /// Each transport protocol is layered according to its own endpoint, and
    /// the data forwarded to this endpoint can be obtained by routing.
    ///
    /// # Example
    ///
    /// ```
    /// use turn_server::router::*;
    /// use turn_rs::StunClass;
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let router = Router::new();
    ///     let (index, mut receiver) = router.get_receiver();
    ///
    ///     router.send(index, StunClass::Channel, &addr, &[1, 2, 3]);
    ///     let ret = receiver.recv().await.unwrap();
    ///     assert_eq!(ret.0, vec![1, 2, 3]);
    ///     assert_eq!(ret.1, StunClass::Channel);
    ///     assert_eq!(ret.2, addr);
    /// }
    /// ```
    pub fn get_receiver(&self) -> (u8, UnboundedReceiver<(Vec<u8>, StunClass, SocketAddr)>) {
        let index = self
            .alloc_index()
            .expect("transport router alloc index failed!");
        let (sender, receiver) = unbounded_channel();
        self.senders.write().insert(index, sender);
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
    /// ```
    /// use turn_server::router::*;
    /// use turn_rs::StunClass;
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let router = Router::new();
    ///     let (index, mut receiver) = router.get_receiver();
    ///
    ///     router.send(index, StunClass::Channel, &addr, &[1, 2, 3]);
    ///     let ret = receiver.recv().await.unwrap();
    ///     assert_eq!(ret.0, vec![1, 2, 3]);
    ///     assert_eq!(ret.1, StunClass::Channel);
    ///     assert_eq!(ret.2, addr);
    /// }
    /// ```
    pub fn send(&self, index: u8, class: StunClass, addr: &SocketAddr, data: &[u8]) {
        let mut is_destroy = false;

        {
            if let Some(sender) = self.senders.read().get(&index) {
                if sender.send((data.to_vec(), class, *addr)).is_err() {
                    is_destroy = true;
                }
            }
        }

        if is_destroy {
            self.remove(index);
        }
    }

    /// delete endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use turn_server::router::*;
    /// use turn_rs::StunClass;
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let router = Router::new();
    ///     let (index, mut receiver) = router.get_receiver();
    ///
    ///     router.send(index, StunClass::Channel, &addr, &[1, 2, 3]);
    ///     let ret = receiver.recv().await.unwrap();
    ///     assert_eq!(ret.0, vec![1, 2, 3]);
    ///     assert_eq!(ret.1, StunClass::Channel);
    ///     assert_eq!(ret.2, addr);
    ///
    ///     router.remove(index);
    ///     assert!(receiver.recv().await.is_none());
    /// }
    /// ```
    pub fn remove(&self, index: u8) {
        if let Some(sender) = self.senders.write().remove(&index) {
            self.free_index(index);
            drop(sender)
        }
    }

    /// alloc a index.
    fn alloc_index(&self) -> Option<u8> {
        let mut bits = self.bits.lock();
        let index = bits.first_one().map(|i| i as u8)?;
        bits.set(index as usize, false);
        Some(index)
    }

    /// free a index from alloced.
    fn free_index(&self, index: u8) {
        self.bits.lock().set(index as usize, true);
    }
}
