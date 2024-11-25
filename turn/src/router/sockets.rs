use std::{collections::BTreeMap, net::SocketAddr, sync::Arc, time::Instant};

use super::ports::capacity;

use ahash::{AHashMap, AHashSet};
use parking_lot::RwLock;
use stun::util::long_key;

/// turn socket session.
#[derive(Clone)]
pub struct Socket {
    pub username: String,
    pub password: String,
    pub channel: Option<u16>,
    pub port: Option<u16>,
    pub secret: Arc<[u8; 16]>,
    pub lifetime: Instant,
    pub expiration: u64,
}

impl Socket {
    /// create socket session.
    ///
    /// socket session from group number and long key.
    pub fn new(realm: &str, username: &str, password: &str) -> Self {
        let secret = Arc::new(long_key(username, password, realm));
        Self {
            username: username.to_string(),
            password: password.to_string(),
            lifetime: Instant::now(),
            expiration: 600,
            channel: None,
            port: None,
            secret,
        }
    }

    /// set the lifetime of the socket.
    ///
    /// delay is to die after the specified second.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::sockets::*;
    ///
    /// let mut socket = Socket::new("test", "test", "test");
    ///
    /// socket.set_lifetime(0);
    /// assert!(socket.is_death());
    ///
    /// socket.set_lifetime(600);
    /// assert!(!socket.is_death());
    /// ```
    pub fn set_lifetime(&mut self, delay: u32) {
        self.expiration = delay as u64;
        self.lifetime = Instant::now();
    }

    /// whether the socket is dead.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::sockets::*;
    ///
    /// let mut socket = Socket::new("test", "test", "test");
    ///
    /// socket.set_lifetime(0);
    /// assert!(socket.is_death());
    ///
    /// socket.set_lifetime(600);
    /// assert!(!socket.is_death());
    /// ```
    pub fn is_death(&self) -> bool {
        self.lifetime.elapsed().as_secs() >= self.expiration
    }

    /// get socket the secret.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::sockets::*;
    ///
    /// let mut socket = Socket::new("test", "test", "test");
    /// let secret = socket.get_secret();
    /// assert_eq!(
    ///     secret.as_slice(),
    ///     &[
    ///         174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16,
    ///         224, 239
    ///     ]
    /// );
    /// ```
    pub fn get_secret(&self) -> Arc<[u8; 16]> {
        self.secret.clone()
    }

    /// posh port in socket.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::sockets::*;
    ///
    /// let mut socket = Socket::new("test", "test", "test");
    ///
    /// socket.set_port(43196);
    /// assert_eq!(socket.port, Some(43196));
    /// ```
    pub fn set_port(&mut self, port: u16) {
        self.port.replace(port);
    }

    /// set channel in socket.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::sockets::*;
    ///
    /// let mut socket = Socket::new("test", "test", "test");
    ///
    /// socket.set_channel(0x4000);
    /// assert_eq!(socket.channel, Some(0x4000));
    /// ```
    pub fn set_channel(&mut self, channel: u16) {
        self.channel.replace(channel);
    }
}

/// socket table.
pub struct Sockets {
    map: RwLock<AHashMap<SocketAddr, Socket>>,
    addrs: RwLock<BTreeMap<String, AHashSet<SocketAddr>>>,
}

impl Default for Sockets {
    fn default() -> Self {
        Self::new()
    }
}

impl Sockets {
    pub fn new() -> Self {
        Self {
            addrs: RwLock::new(BTreeMap::new()),
            map: RwLock::new(AHashMap::with_capacity(capacity())),
        }
    }

    /// get users name and address.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// assert_eq!(sockets.get_users(0, 10), vec![]);
    /// ```
    pub fn get_users(&self, skip: usize, limit: usize) -> Vec<(String, Vec<SocketAddr>)> {
        self.addrs
            .read()
            .iter()
            .skip(skip)
            .take(limit)
            .map(|(k, v)| (k.clone(), v.iter().copied().collect()))
            .collect()
    }

    /// get socket from name.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// sockets.insert(&addr, "test", "test", "test");
    ///
    /// let socket = sockets.get_socket(&addr).unwrap();
    /// assert_eq!(socket.username.as_str(), "test");
    /// assert_eq!(socket.password.as_str(), "test");
    /// assert_eq!(
    ///     socket.secret.as_slice(),
    ///     &[
    ///         174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16,
    ///         224, 239
    ///     ]
    /// );
    /// assert_eq!(socket.channel, None);
    /// assert_eq!(socket.port, None);
    /// ```
    pub fn get_socket(&self, a: &SocketAddr) -> Option<Socket> {
        self.map.read().get(a).cloned()
    }

    /// get password from address.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// sockets.insert(&addr, "test", "test", "test");
    ///
    /// let secret = sockets.get_secret(&addr).unwrap();
    /// assert_eq!(
    ///     secret.as_slice(),
    ///     &[
    ///         174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16,
    ///         224, 239
    ///     ]
    /// );
    /// ```
    pub fn get_secret(&self, a: &SocketAddr) -> Option<Arc<[u8; 16]>> {
        self.map.read().get(a).map(|n| n.get_secret())
    }

    /// insert socket in socket table.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// sockets.insert(&addr, "test", "test", "test");
    ///
    /// let socket = sockets.get_socket(&addr).unwrap();
    /// assert_eq!(socket.username.as_str(), "test");
    /// assert_eq!(socket.password.as_str(), "test");
    /// assert_eq!(
    ///     socket.secret.as_slice(),
    ///     &[
    ///         174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16,
    ///         224, 239
    ///     ]
    /// );
    /// assert_eq!(socket.channel, None);
    /// assert_eq!(socket.port, None);
    /// ```
    pub fn insert(
        &self,
        addr: &SocketAddr,
        realm: &str,
        username: &str,
        password: &str,
    ) -> Option<Arc<[u8; 16]>> {
        let socket = Socket::new(realm, username, password);
        let secret = socket.get_secret();
        let mut addrs = self.addrs.write();
        self.map.write().insert(*addr, socket);

        addrs
            .entry(username.to_string())
            .or_insert_with(|| AHashSet::with_capacity(5))
            .insert(*addr);
        Some(secret)
    }

    /// Get whether the current socket has been assigned a port.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// sockets.insert(&addr, "test", "test", "test");
    ///
    /// assert_eq!(sockets.get_port(&addr), None);
    /// assert!(sockets.set_port(&addr, 6000).is_some());
    /// assert_eq!(sockets.get_port(&addr), Some(6000));
    /// ```
    pub fn get_port(&self, addr: &SocketAddr) -> Option<u16> {
        self.map.read().get(addr)?.port
    }

    /// Get whether the current socket has been assigned a channel.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// sockets.insert(&addr, "test", "test", "test");
    ///
    /// assert_eq!(sockets.get_channel(&addr), None);
    /// assert!(sockets.set_channel(&addr, 6000).is_some());
    /// assert_eq!(sockets.get_channel(&addr), Some(6000));
    /// ```
    pub fn get_channel(&self, addr: &SocketAddr) -> Option<u16> {
        self.map.read().get(addr)?.channel
    }

    /// set port to socket.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// sockets.insert(&addr, "test", "test", "test");
    ///
    /// assert!(sockets.set_port(&addr, 60000).is_some());
    ///
    /// let socket = sockets.get_socket(&addr).unwrap();
    /// assert_eq!(socket.username.as_str(), "test");
    /// assert_eq!(socket.password.as_str(), "test");
    /// assert_eq!(
    ///     socket.secret.as_slice(),
    ///     &[
    ///         174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16,
    ///         224, 239
    ///     ]
    /// );
    /// assert_eq!(socket.channel, None);
    /// assert_eq!(socket.port, Some(60000));
    /// ```
    pub fn set_port(&self, addr: &SocketAddr, port: u16) -> Option<()> {
        self.map.write().get_mut(addr)?.set_port(port);
        Some(())
    }

    /// set channel to socket.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// sockets.insert(&addr, "test", "test", "test");
    ///
    /// assert!(sockets.set_channel(&addr, 0x4000).is_some());
    ///
    /// let socket = sockets.get_socket(&addr).unwrap();
    /// assert_eq!(socket.username.as_str(), "test");
    /// assert_eq!(socket.password.as_str(), "test");
    /// assert_eq!(
    ///     socket.secret.as_slice(),
    ///     &[
    ///         174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16,
    ///         224, 239
    ///     ]
    /// );
    /// assert_eq!(socket.channel, Some(0x4000));
    /// assert_eq!(socket.port, None);
    /// ```
    pub fn set_channel(&self, addr: &SocketAddr, channel: u16) -> Option<()> {
        self.map.write().get_mut(addr)?.set_channel(channel);
        Some(())
    }

    /// set lifetime to socket.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// sockets.insert(&addr, "test", "test", "test");
    ///
    /// assert!(sockets.set_lifetime(&addr, 600).is_some());
    ///
    /// let socket = sockets.get_socket(&addr).unwrap();
    /// assert_eq!(socket.username.as_str(), "test");
    /// assert_eq!(socket.password.as_str(), "test");
    /// assert_eq!(
    ///     socket.secret.as_slice(),
    ///     &[
    ///         174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16,
    ///         224, 239
    ///     ]
    /// );
    /// assert_eq!(socket.channel, None);
    /// assert_eq!(socket.port, None);
    /// assert!(!socket.is_death());
    ///
    /// assert!(sockets.set_lifetime(&addr, 0).is_some());
    ///
    /// let socket = sockets.get_socket(&addr).unwrap();
    /// assert_eq!(socket.username.as_str(), "test");
    /// assert_eq!(socket.password.as_str(), "test");
    /// assert_eq!(
    ///     socket.secret.as_slice(),
    ///     &[
    ///         174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16,
    ///         224, 239
    ///     ]
    /// );
    /// assert_eq!(socket.channel, None);
    /// assert_eq!(socket.port, None);
    /// assert!(socket.is_death());
    /// ```
    pub fn set_lifetime(&self, a: &SocketAddr, delay: u32) -> Option<()> {
        self.map.write().get_mut(a)?.set_lifetime(delay);
        Some(())
    }

    /// remove socket from address.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// sockets.insert(&addr, "test", "test", "test");
    ///
    /// let socket = sockets.get_socket(&addr).unwrap();
    /// assert_eq!(socket.username.as_str(), "test");
    /// assert_eq!(socket.password.as_str(), "test");
    /// assert_eq!(
    ///     socket.secret.as_slice(),
    ///     &[
    ///         174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16,
    ///         224, 239
    ///     ]
    /// );
    /// assert_eq!(socket.channel, None);
    /// assert_eq!(socket.port, None);
    ///
    /// assert!(sockets.remove(&addr).is_some());
    /// assert!(sockets.get_socket(&addr).is_none());
    /// ```
    pub fn remove(&self, a: &SocketAddr) -> Option<Socket> {
        let mut user_addrs = self.addrs.write();
        let socket = self.map.write().remove(a)?;
        let addrs = user_addrs.get_mut(&socket.username)?;
        if addrs.len() == 1 {
            user_addrs.remove(&socket.username)?;
        } else {
            addrs.remove(a);
        }

        Some(socket)
    }

    /// get socket name bound address.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let addr1 = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    ///
    /// sockets.insert(&addr, "test", "test", "test");
    /// sockets.insert(&addr1, "test", "test", "test");
    ///
    /// let ret = sockets.get_addrs("test");
    ///
    /// assert_eq!(ret.len(), 2);
    /// assert!(ret[0] == addr || ret[0] == addr1);
    /// assert!(ret[1] == addr || ret[1] == addr1);
    /// ```
    pub fn get_addrs(&self, username: &str) -> Vec<SocketAddr> {
        self.addrs
            .read()
            .get(username)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect()
    }

    /// get death socket.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::router::sockets::*;
    ///
    /// let sockets = Sockets::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// sockets.insert(&addr, "test", "test", "test");
    ///
    /// assert!(sockets.set_lifetime(&addr, 600).is_some());
    /// assert_eq!(sockets.get_deaths(), vec![]);
    ///
    /// assert!(sockets.set_lifetime(&addr, 0).is_some());
    /// assert_eq!(sockets.get_deaths(), vec![addr]);
    /// ```
    pub fn get_deaths(&self) -> Vec<SocketAddr> {
        self.map
            .read()
            .iter()
            .filter(|(_, v)| v.is_death())
            .map(|(k, _)| *k)
            .collect::<Vec<SocketAddr>>()
    }
}
