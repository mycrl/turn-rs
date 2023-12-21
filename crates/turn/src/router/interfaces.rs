use super::ports::capacity;

use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use ahash::AHashMap;

#[derive(Clone, Copy, Debug)]
pub struct Interface {
    pub addr: SocketAddr,
    pub external: SocketAddr,
}

pub struct Interfaces {
    map: RwLock<AHashMap<SocketAddr, Arc<Interface>>>,
}

impl Default for Interfaces {
    fn default() -> Self {
        Self {
            map: RwLock::new(AHashMap::with_capacity(capacity())),
        }
    }
}

impl Interfaces {
    /// add interface from addr.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_rs::router::interfaces::*;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let interface = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let interfaces = Interfaces::default();
    ///
    /// interfaces.insert(addr, interface, interface);
    /// let ret = interfaces.get(&addr).unwrap();
    /// assert_eq!(ret.addr, interface);
    /// assert_eq!(ret.external, interface);
    /// ```
    pub fn insert(&self, addr: SocketAddr, interface: SocketAddr, external: SocketAddr) {
        self.map.write().unwrap().insert(
            addr,
            Arc::new(Interface {
                addr: interface,
                external,
            }),
        );
    }

    /// get interface from addr.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_rs::router::interfaces::*;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let interface = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let interfaces = Interfaces::default();
    ///
    /// interfaces.insert(addr, interface, interface);
    /// let ret = interfaces.get(&addr).unwrap();
    /// assert_eq!(ret.addr, interface);
    /// assert_eq!(ret.external, interface);
    /// ```
    pub fn get(&self, addr: &SocketAddr) -> Option<Interface> {
        self.map
            .read()
            .unwrap()
            .get(addr)
            .map(|item| *item.as_ref())
    }

    /// get interface ref from addr.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::interfaces::*;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let interface = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let interfaces = Interfaces::default();
    ///
    /// interfaces.insert(addr, interface, interface);
    /// let ret = interfaces.get_ref(&addr).unwrap();
    /// assert_eq!(ret.addr, interface);
    /// assert_eq!(ret.external, interface);
    /// ```
    pub fn get_ref(&self, addr: &SocketAddr) -> Option<Arc<Interface>> {
        self.map.read().unwrap().get(addr).cloned()
    }

    /// remove interface from addr.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::interfaces::*;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let interface = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let interfaces = Interfaces::default();
    ///
    /// interfaces.insert(addr, interface, interface);
    /// let ret = interfaces.get(&addr).unwrap();
    /// assert_eq!(ret.addr, interface);
    /// assert_eq!(ret.external, interface);
    ///
    /// interfaces.remove(&addr);
    ///
    /// let ret = interfaces.get(&addr);
    /// assert!(ret.is_none());
    /// ```
    pub fn remove(&self, addr: &SocketAddr) {
        self.map.write().unwrap().remove(addr);
    }
}
