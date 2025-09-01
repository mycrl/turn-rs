use super::OutboundType;

use std::{net::SocketAddr, sync::Arc};

use ahash::AHashMap;
use bytes::Bytes;
use parking_lot::RwLock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

type Receiver = UnboundedSender<(Bytes, OutboundType, SocketAddr)>;

/// Handles packet forwarding between transport protocols.
#[derive(Clone)]
pub struct Exchanger(Arc<RwLock<AHashMap<SocketAddr, Receiver>>>);

impl Default for Exchanger {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(AHashMap::with_capacity(1024))))
    }
}

impl Exchanger {
    /// Get the socket reader for the route.
    ///
    /// Each transport protocol is layered according to its own socket, and
    /// the data forwarded to this socket can be obtained by routing.
    pub fn get_receiver(
        &self,
        interface: SocketAddr,
    ) -> UnboundedReceiver<(Bytes, OutboundType, SocketAddr)> {
        let (sender, receiver) = unbounded_channel();
        self.0.write().insert(interface, sender);
        receiver
    }

    /// Send data to dispatcher.
    ///
    /// By specifying the socket identifier and destination address, the route
    /// is forwarded to the corresponding socket. However, it should be noted
    /// that calling this function will not notify whether the socket exists.
    /// If it does not exist, the data will be discarded by default.
    pub fn send(&self, interface: &SocketAddr, ty: OutboundType, addr: &SocketAddr, data: Bytes) {
        let mut is_destroy = false;

        {
            if let Some(sender) = self.0.read().get(interface) {
                if sender.send((data, ty, *addr)).is_err() {
                    is_destroy = true;
                }
            }
        }

        if is_destroy {
            self.remove(interface);
        }
    }

    /// delete socket.
    pub fn remove(&self, interface: &SocketAddr) {
        drop(self.0.write().remove(interface))
    }
}
