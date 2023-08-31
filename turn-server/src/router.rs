use std::{net::SocketAddr, sync::RwLock};

use ahash::AHashMap;
use tokio::sync::mpsc::*;
use turn_rs::StunClass;

type Receiver = UnboundedSender<(Vec<u8>, StunClass, SocketAddr)>;

/// Handles packet forwarding between transport protocols.
#[derive(Default)]
pub struct Router {
    senders: RwLock<AHashMap<SocketAddr, Receiver>>,
}

impl Router {
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
    ///     let router = Router::default();
    ///     let mut receiver = router.get_receiver(addr);
    ///
    ///     router.send(&addr, StunClass::Channel, &addr, &[1, 2, 3]);
    ///     let ret = receiver.recv().await.unwrap();
    ///     assert_eq!(ret.0, vec![1, 2, 3]);
    ///     assert_eq!(ret.1, StunClass::Channel);
    ///     assert_eq!(ret.2, addr);
    /// }
    /// ```
    pub fn get_receiver(
        &self,
        interface: SocketAddr,
    ) -> UnboundedReceiver<(Vec<u8>, StunClass, SocketAddr)> {
        let (sender, receiver) = unbounded_channel();
        self.senders.write().unwrap().insert(interface, sender);
        receiver
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
    ///     let router = Router::default();
    ///     let mut receiver = router.get_receiver(addr);
    ///
    ///     router.send(&addr, StunClass::Channel, &addr, &[1, 2, 3]);
    ///     let ret = receiver.recv().await.unwrap();
    ///     assert_eq!(ret.0, vec![1, 2, 3]);
    ///     assert_eq!(ret.1, StunClass::Channel);
    ///     assert_eq!(ret.2, addr);
    /// }
    /// ```
    pub fn send(&self, interface: &SocketAddr, class: StunClass, addr: &SocketAddr, data: &[u8]) {
        let mut is_destroy = false;

        {
            if let Some(sender) = self.senders.read().unwrap().get(&interface) {
                if sender.send((data.to_vec(), class, *addr)).is_err() {
                    is_destroy = true;
                }
            }
        }

        if is_destroy {
            self.remove(interface);
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
    ///     let router = Router::default();
    ///     let mut receiver = router.get_receiver(addr);
    ///
    ///     router.send(&addr, StunClass::Channel, &addr, &[1, 2, 3]);
    ///     let ret = receiver.recv().await.unwrap();
    ///     assert_eq!(ret.0, vec![1, 2, 3]);
    ///     assert_eq!(ret.1, StunClass::Channel);
    ///     assert_eq!(ret.2, addr);
    ///
    ///     router.remove(&addr);
    ///     assert!(receiver.recv().await.is_none());
    /// }
    /// ```
    pub fn remove(&self, interface: &SocketAddr) {
        drop(self.senders.write().unwrap().remove(&interface))
    }
}
