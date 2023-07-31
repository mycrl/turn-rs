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
    /// interfaces.insert(addr, interface);
    /// let ret = interfaces.get(&addr);
    /// assert_eq!(ret, Some(interface));
    /// ```
    pub fn insert(&self, addr: SocketAddr, interface: SocketAddr) {
        self.map.write().insert(addr, Arc::new(interface));
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
    /// interfaces.insert(addr, interface);
    /// let ret = interfaces.get(&addr);
    /// assert_eq!(ret, Some(interface));
    /// ```
    pub fn get(&self, addr: &SocketAddr) -> Option<SocketAddr> {
        self.map.read().get(addr).map(|item| *item.as_ref())
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
    /// interfaces.insert(addr, interface);
    /// let ret = interfaces.get_ref(&addr);
    /// assert_eq!(ret, Some(Arc::new(interface)));
    /// ```
    pub fn get_ref(&self, addr: &SocketAddr) -> Option<Arc<SocketAddr>> {
        self.map.read().get(addr).cloned()
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
    /// interfaces.insert(addr, interface);
    /// let ret = interfaces.get(&addr);
    /// assert_eq!(ret, Some(interface));
    ///
    /// interfaces.remove(&addr);
    ///
    /// let ret = interfaces.get(&addr);
    /// assert_eq!(ret, None);
    /// ```
    pub fn remove(&self, addr: &SocketAddr) {
        self.map.write().remove(addr);
    }
}
