use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Instant,
};

use super::ports::capacity;

use ahash::{AHashMap, AHashSet};
use faster_stun::util::long_key;

/// turn node session.
#[derive(Clone)]
pub struct Node {
    pub channels: Vec<u16>,
    pub ports: Vec<u16>,
    pub timer: Instant,
    pub lifetime: u64,
    pub secret: Arc<[u8; 16]>,
    pub username: String,
    pub password: String,
}

impl Node {
    /// create node session.
    ///
    /// node session from group number and long key.
    pub fn new(realm: &str, username: &str, password: &str) -> Self {
        let secret = Arc::new(long_key(username, password, realm));
        Self {
            channels: Vec::with_capacity(5),
            ports: Vec::with_capacity(10),
            username: username.to_string(),
            password: password.to_string(),
            timer: Instant::now(),
            lifetime: 600,
            secret,
        }
    }

    /// set the lifetime of the node.
    ///
    /// delay is to die after the specified second.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    ///
    /// let mut node = Node::new("test", "test", "test");
    ///
    /// node.set_lifetime(0);
    /// assert!(node.is_death());
    ///
    /// node.set_lifetime(600);
    /// assert!(!node.is_death());
    /// ```
    pub fn set_lifetime(&mut self, delay: u32) {
        self.lifetime = delay as u64;
        self.timer = Instant::now();
    }

    /// whether the node is dead.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    ///
    /// let mut node = Node::new("test", "test", "test");
    ///
    /// node.set_lifetime(0);
    /// assert!(node.is_death());
    ///
    /// node.set_lifetime(600);
    /// assert!(!node.is_death());
    /// ```
    pub fn is_death(&self) -> bool {
        self.timer.elapsed().as_secs() >= self.lifetime
    }

    /// get node the secret.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    ///
    /// let mut node = Node::new("test", "test", "test");
    /// let secret = node.get_secret();
    /// assert_eq!(secret.as_slice(), &[174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239]);
    /// ```
    pub fn get_secret(&self) -> Arc<[u8; 16]> {
        self.secret.clone()
    }

    /// posh port in node.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    ///
    /// let mut node = Node::new("test", "test", "test");
    ///
    /// node.push_port(43196);
    /// assert_eq!(&node.ports, &[43196]);
    /// ```
    pub fn push_port(&mut self, port: u16) {
        if !self.ports.contains(&port) {
            self.ports.push(port);
        }
    }

    /// push channel in node.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    ///
    /// let mut node = Node::new("test", "test", "test");
    ///
    /// node.push_channel(0x4000);
    /// assert_eq!(&node.channels, &[0x4000]);
    /// ```
    pub fn push_channel(&mut self, channel: u16) {
        if !self.channels.contains(&channel) {
            self.channels.push(channel);
        }
    }
}

/// node table.
pub struct Nodes {
    map: RwLock<AHashMap<SocketAddr, Node>>,
    addrs: RwLock<BTreeMap<String, AHashSet<SocketAddr>>>,
}

impl Default for Nodes {
    fn default() -> Self {
        Self::new()
    }
}

impl Nodes {
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
    /// use turn_rs::router::nodes::*;
    ///
    /// let nodes = Nodes::new();
    /// assert_eq!(nodes.get_users(0, 10), vec![]);
    /// ```
    pub fn get_users(&self, skip: usize, limit: usize) -> Vec<(String, Vec<SocketAddr>)> {
        self.addrs
            .read()
            .unwrap()
            .iter()
            .skip(skip)
            .take(limit)
            .map(|(k, v)| (k.clone(), v.iter().copied().collect()))
            .collect()
    }

    /// get node from name.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    /// use std::net::SocketAddr;
    ///
    /// let nodes = Nodes::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// nodes.insert(&addr, "test", "test", "test");
    ///
    /// let node = nodes.get_node(&addr).unwrap();
    /// assert_eq!(node.username.as_str(), "test");
    /// assert_eq!(node.password.as_str(), "test");
    /// assert_eq!(node.secret.as_slice(), &[174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239]);
    /// assert_eq!(node.channels.len(), 0);
    /// assert_eq!(node.ports.len(), 0);
    /// ```
    pub fn get_node(&self, a: &SocketAddr) -> Option<Node> {
        self.map.read().unwrap().get(a).cloned()
    }

    /// get password from address.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    /// use std::net::SocketAddr;
    ///
    /// let nodes = Nodes::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// nodes.insert(&addr, "test", "test", "test");
    ///
    /// let secret = nodes.get_secret(&addr).unwrap();
    /// assert_eq!(secret.as_slice(), &[174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239]);
    /// ```
    pub fn get_secret(&self, a: &SocketAddr) -> Option<Arc<[u8; 16]>> {
        self.map.read().unwrap().get(a).map(|n| n.get_secret())
    }

    /// insert node in node table.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    /// use std::net::SocketAddr;
    ///
    /// let nodes = Nodes::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// nodes.insert(&addr, "test", "test", "test");
    ///
    /// let node = nodes.get_node(&addr).unwrap();
    /// assert_eq!(node.username.as_str(), "test");
    /// assert_eq!(node.password.as_str(), "test");
    /// assert_eq!(node.secret.as_slice(), &[174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239]);
    /// assert_eq!(node.channels.len(), 0);
    /// assert_eq!(node.ports.len(), 0);
    /// ```
    pub fn insert(
        &self,
        addr: &SocketAddr,
        realm: &str,
        username: &str,
        password: &str,
    ) -> Option<Arc<[u8; 16]>> {
        let node = Node::new(realm, username, password);
        let pwd = node.get_secret();
        let mut addrs = self.addrs.write().unwrap();
        self.map.write().unwrap().insert(*addr, node);

        addrs
            .entry(username.to_string())
            .or_insert_with(|| AHashSet::with_capacity(5))
            .insert(*addr);
        Some(pwd)
    }

    /// push port to node.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    /// use std::net::SocketAddr;
    ///
    /// let nodes = Nodes::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// nodes.insert(&addr, "test", "test", "test");
    ///
    /// assert!(nodes.push_port(&addr, 60000).is_some());
    ///
    /// let node = nodes.get_node(&addr).unwrap();
    /// assert_eq!(node.username.as_str(), "test");
    /// assert_eq!(node.password.as_str(), "test");
    /// assert_eq!(node.secret.as_slice(), &[174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239]);
    /// assert_eq!(node.channels, vec![]);
    /// assert_eq!(node.ports, vec![60000]);
    /// ```
    pub fn push_port(&self, a: &SocketAddr, port: u16) -> Option<()> {
        self.map.write().unwrap().get_mut(a)?.push_port(port);
        Some(())
    }

    /// push channel to node.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    /// use std::net::SocketAddr;
    ///
    /// let nodes = Nodes::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// nodes.insert(&addr, "test", "test", "test");
    ///
    /// assert!(nodes.push_channel(&addr, 0x4000).is_some());
    ///
    /// let node = nodes.get_node(&addr).unwrap();
    /// assert_eq!(node.username.as_str(), "test");
    /// assert_eq!(node.password.as_str(), "test");
    /// assert_eq!(node.secret.as_slice(), &[174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239]);
    /// assert_eq!(node.channels, vec![0x4000]);
    /// assert_eq!(node.ports, vec![]);
    /// ```
    pub fn push_channel(&self, a: &SocketAddr, channel: u16) -> Option<()> {
        self.map.write().unwrap().get_mut(a)?.push_channel(channel);
        Some(())
    }

    /// set lifetime to node.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    /// use std::net::SocketAddr;
    ///
    /// let nodes = Nodes::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// nodes.insert(&addr, "test", "test", "test");
    ///
    /// assert!(nodes.set_lifetime(&addr, 600).is_some());
    ///
    /// let node = nodes.get_node(&addr).unwrap();
    /// assert_eq!(node.username.as_str(), "test");
    /// assert_eq!(node.password.as_str(), "test");
    /// assert_eq!(node.secret.as_slice(), &[174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239]);
    /// assert_eq!(node.channels, vec![]);
    /// assert_eq!(node.ports, vec![]);
    /// assert!(!node.is_death());
    ///
    /// assert!(nodes.set_lifetime(&addr, 0).is_some());
    ///
    /// let node = nodes.get_node(&addr).unwrap();
    /// assert_eq!(node.username.as_str(), "test");
    /// assert_eq!(node.password.as_str(), "test");
    /// assert_eq!(node.secret.as_slice(), &[174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239]);
    /// assert_eq!(node.channels, vec![]);
    /// assert_eq!(node.ports, vec![]);
    /// assert!(node.is_death());
    /// ```
    pub fn set_lifetime(&self, a: &SocketAddr, delay: u32) -> Option<()> {
        self.map.write().unwrap().get_mut(a)?.set_lifetime(delay);
        Some(())
    }

    /// remove node from address.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    /// use std::net::SocketAddr;
    ///
    /// let nodes = Nodes::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// nodes.insert(&addr, "test", "test", "test");
    ///
    /// let node = nodes.get_node(&addr).unwrap();
    /// assert_eq!(node.username.as_str(), "test");
    /// assert_eq!(node.password.as_str(), "test");
    /// assert_eq!(node.secret.as_slice(), &[174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239]);
    /// assert_eq!(node.channels, vec![]);
    /// assert_eq!(node.ports, vec![]);
    ///
    /// assert!(nodes.remove(&addr).is_some());
    /// assert!(nodes.get_node(&addr).is_none());
    /// ```
    pub fn remove(&self, a: &SocketAddr) -> Option<Node> {
        let mut user_addrs = self.addrs.write().unwrap();
        let node = self.map.write().unwrap().remove(a)?;
        let addrs = user_addrs.get_mut(&node.username)?;
        if addrs.len() == 1 {
            user_addrs.remove(&node.username)?;
        } else {
            addrs.remove(a);
        }

        Some(node)
    }

    /// get node name bound address.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    /// use std::net::SocketAddr;
    ///
    /// let nodes = Nodes::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let addr1 = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    ///
    /// nodes.insert(&addr, "test", "test", "test");
    /// nodes.insert(&addr1, "test", "test", "test");
    ///
    /// let ret = nodes.get_addrs("test");
    ///
    /// assert_eq!(ret.len(), 2);
    /// assert!(ret[0] == addr || ret[0] == addr1);
    /// assert!(ret[1] == addr || ret[1] == addr1);
    /// ```
    pub fn get_addrs(&self, u: &str) -> Vec<SocketAddr> {
        self.addrs
            .read()
            .unwrap()
            .get(u)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect()
    }

    /// get death node.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nodes::*;
    /// use std::net::SocketAddr;
    ///
    /// let nodes = Nodes::new();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// nodes.insert(&addr, "test", "test", "test");
    ///
    /// assert!(nodes.set_lifetime(&addr, 600).is_some());
    /// assert_eq!(nodes.get_deaths(), vec![]);
    ///
    /// assert!(nodes.set_lifetime(&addr, 0).is_some());
    /// assert_eq!(nodes.get_deaths(), vec![addr]);
    /// ```
    pub fn get_deaths(&self) -> Vec<SocketAddr> {
        self.map
            .read()
            .unwrap()
            .iter()
            .filter(|(_, v)| v.is_death())
            .map(|(k, _)| *k)
            .collect::<Vec<SocketAddr>>()
    }
}
