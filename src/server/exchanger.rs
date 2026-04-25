use std::sync::Arc;

use ahash::{HashMap, HashMapExt};
use parking_lot::RwLock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::{server::memory_pool::Buffer, service::session::Identifier};

pub type ExchangerSender = UnboundedSender<Buffer>;

/// Handles packet forwarding between transport protocols.
#[derive(Clone)]
pub struct Exchanger(Arc<RwLock<HashMap<Identifier, ExchangerSender>>>);

impl Default for Exchanger {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(HashMap::with_capacity(1024))))
    }
}

impl Exchanger {
    /// Get the socket reader for the route.
    ///
    /// Each transport protocol is layered according to its own socket, and
    /// the data forwarded to this socket can be obtained by routing.
    pub fn get_receiver(&self, id: Identifier) -> UnboundedReceiver<Buffer> {
        let (sender, receiver) = unbounded_channel();
        self.0.write().insert(id, sender);

        receiver
    }

    /// Send data to dispatcher.
    ///
    /// By specifying the socket identifier and destination address, the route
    /// is forwarded to the corresponding socket. However, it should be noted
    /// that calling this function will not notify whether the socket exists.
    /// If it does not exist, the data will be discarded by default.
    pub fn send(&self, id: &Identifier, bytes: Buffer) {
        let mut is_destroy = false;

        {
            if let Some(sender) = self.0.read().get(id)
                && sender.send(bytes).is_err()
            {
                is_destroy = true;
            }
        }

        if is_destroy {
            self.remove(id);
        }
    }

    /// delete socket.
    pub fn remove(&self, id: &Identifier) {
        drop(self.0.write().remove(id))
    }
}
