use super::ports::capacity;

use ahash::AHashMap;

use parking_lot::RwLock;
use std::net::SocketAddr;

/// channels table.
pub struct Channels {
    map: RwLock<AHashMap<(SocketAddr, u16), SocketAddr>>,
}

impl Default for Channels {
    fn default() -> Self {
        Self::new()
    }
}

impl Channels {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(AHashMap::with_capacity(capacity())),
        }
    }

    /// get bind address.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::channels::*;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let channels = Channels::new();
    ///
    /// channels.insert(&addr, 43159, &peer).unwrap();
    /// channels.insert(&peer, 43160, &addr).unwrap();
    ///
    /// assert_eq!(channels.get_bind(&addr, 43159).unwrap(), peer);
    /// ```
    pub fn get_bind(&self, addr: &SocketAddr, channel: u16) -> Option<SocketAddr> {
        self.map.read().get(&(*addr, channel)).cloned()
    }

    /// insert address for peer address to channel table.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::channels::*;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let channels = Channels::new();
    ///
    /// channels.insert(&addr, 43159, &peer).unwrap();
    /// channels.insert(&peer, 43160, &addr).unwrap();
    ///
    /// assert_eq!(channels.get_bind(&addr, 43159).unwrap(), peer);
    /// ```
    pub fn insert(&self, addr: &SocketAddr, number: u16, peer: &SocketAddr) -> Option<()> {
        self.map.write().insert((*addr, number), *peer);
        Some(())
    }

    /// remove channel allocate in channel table.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::channels::*;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let channels = Channels::new();
    ///
    /// channels.insert(&addr, 43159, &peer).unwrap();
    /// channels.insert(&peer, 43160, &addr).unwrap();
    ///
    /// channels.remove(addr, 43159);
    /// assert_eq!(channels.get_bind(&addr, 43159), None);
    /// assert_eq!(channels.get_bind(&peer, 43160), Some(addr));
    ///
    /// channels.remove(peer, 43160);
    /// assert_eq!(channels.get_bind(&addr, 43159), None);
    /// assert_eq!(channels.get_bind(&peer, 43160), None);
    /// ```
    pub fn remove(&self, addr: SocketAddr, number: u16) {
        self.map.write().remove(&(addr, number));
    }
}
