use super::ports::capacity;

use std::net::SocketAddr;
use std::sync::Arc;

use ahash::AHashMap;
use parking_lot::RwLock;

pub struct Interfaces {
    map: RwLock<AHashMap<SocketAddr, Arc<SocketAddr>>>,
}

impl Default for Interfaces {
    fn default() -> Self {
        Self {
            map: RwLock::new(AHashMap::with_capacity(capacity())),
        }
    }
}

impl Interfaces {
    pub fn insert(&self, addr: SocketAddr, interface: SocketAddr) {
        self.map.write().insert(addr, Arc::new(interface));
    }

    pub fn get(&self, addr: &SocketAddr) -> Option<SocketAddr> {
        self.map.read().get(addr).map(|item| *item.as_ref())
    }

    pub fn get_ref(&self, addr: &SocketAddr) -> Option<Arc<SocketAddr>> {
        self.map.read().get(addr).cloned()
    }

    pub fn remove(&self, addr: &SocketAddr) {
        self.map.write().remove(addr);
    }
}
