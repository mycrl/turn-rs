use std::{net::SocketAddr, sync::Arc};

use ahash::AHashMap;
use parking_lot::RwLock;
use tokio::sync::mpsc::*;
use turn::ResponseMethod;

type Receiver = UnboundedSender<(Vec<u8>, ResponseMethod, SocketAddr)>;

/// Handles packet forwarding between transport protocols.
#[derive(Clone)]
pub struct Router(Arc<RwLock<AHashMap<SocketAddr, Receiver>>>);

impl Default for Router {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(AHashMap::with_capacity(1024))))
    }
}

impl Router {
    /// Get the socket reader for the route.
    ///
    /// Each transport protocol is layered according to its own socket, and
    /// the data forwarded to this socket can be obtained by routing.
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::ResponseMethod;
    /// use turn_server::router::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let router = Router::default();
    ///     let mut receiver = router.get_receiver(addr);
    ///
    ///     router.send(&addr, ResponseMethod::ChannelData, &addr, &[1, 2, 3]);
    ///     let ret = receiver.recv().await.unwrap();
    ///     assert_eq!(ret.0, vec![1, 2, 3]);
    ///     assert_eq!(ret.1, ResponseMethod::ChannelData);
    ///     assert_eq!(ret.2, addr);
    /// }
    /// ```
    pub fn get_receiver(&self, interface: SocketAddr) -> UnboundedReceiver<(Vec<u8>, ResponseMethod, SocketAddr)> {
        let (sender, receiver) = unbounded_channel();
        self.0.write().insert(interface, sender);
        receiver
    }

    /// Send data to router.
    ///
    /// By specifying the socket identifier and destination address, the route
    /// is forwarded to the corresponding socket. However, it should be noted
    /// that calling this function will not notify whether the socket exists.
    /// If it does not exist, the data will be discarded by default.
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::ResponseMethod;
    /// use turn_server::router::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let router = Router::default();
    ///     let mut receiver = router.get_receiver(addr);
    ///
    ///     router.send(&addr, ResponseMethod::ChannelData, &addr, &[1, 2, 3]);
    ///     let ret = receiver.recv().await.unwrap();
    ///     assert_eq!(ret.0, vec![1, 2, 3]);
    ///     assert_eq!(ret.1, ResponseMethod::ChannelData);
    ///     assert_eq!(ret.2, addr);
    /// }
    /// ```
    pub fn send(&self, interface: &SocketAddr, method: ResponseMethod, addr: &SocketAddr, data: &[u8]) {
        let mut is_destroy = false;

        {
            if let Some(sender) = self.0.read().get(interface) {
                if sender.send((data.to_vec(), method, *addr)).is_err() {
                    is_destroy = true;
                }
            }
        }

        if is_destroy {
            self.remove(interface);
        }
    }

    /// delete socket.
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::ResponseMethod;
    /// use turn_server::router::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let router = Router::default();
    ///     let mut receiver = router.get_receiver(addr);
    ///
    ///     router.send(&addr, ResponseMethod::ChannelData, &addr, &[1, 2, 3]);
    ///     let ret = receiver.recv().await.unwrap();
    ///     assert_eq!(ret.0, vec![1, 2, 3]);
    ///     assert_eq!(ret.1, ResponseMethod::ChannelData);
    ///     assert_eq!(ret.2, addr);
    ///
    ///     router.remove(&addr);
    ///     assert!(receiver.recv().await.is_none());
    /// }
    /// ```
    pub fn remove(&self, interface: &SocketAddr) {
        drop(self.0.write().remove(interface))
    }
}
