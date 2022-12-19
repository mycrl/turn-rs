use super::{
    ports::capacity,
    Addr,
};

use tokio::{
    sync::RwLock,
    time::Instant,
};

use std::{
    collections::HashMap,
    collections::HashSet,
    sync::Arc,
    net::SocketAddr,
};

/// turn node session.
///
/// * the authentication information.
/// * the port bind table.
/// * the channel alloc table.
/// * the group number.
/// * the time-to-expiry for each relayed transport address.
#[derive(Clone)]
pub struct Node {
    pub channels: Vec<u16>,
    pub ports: Vec<u16>,
    pub timer: Instant,
    pub lifetime: u64,
    pub password: Arc<[u8; 16]>,
    pub username: String,
}

impl Node {
    /// create node session.
    ///
    /// node session from group number and long key.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// // Node::new(0, key.clone());
    /// ```
    pub fn new(username: String, password: [u8; 16]) -> Self {
        Self {
            channels: Vec::with_capacity(5),
            ports: Vec::with_capacity(10),
            password: Arc::new(password),
            timer: Instant::now(),
            lifetime: 600,
            username,
        }
    }

    /// set the lifetime of the node.
    ///
    /// delay is to die after the specified second.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let mut node = Node::new(0, key.clone());
    /// node.set_lifetime(600);
    /// ```
    pub fn set_lifetime(&mut self, delay: u32) {
        self.lifetime = delay as u64;
        self.timer = Instant::now();
    }

    /// whether the node is dead.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let mut node = Node::new(0, key.clone());
    /// node.set_lifetime(600);
    /// assert!(!node.is_death());
    /// ```
    pub fn is_death(&self) -> bool {
        self.timer.elapsed().as_secs() >= self.lifetime
    }

    /// get node the password.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let node = Node::new(0, key.clone());
    /// assert_eq!(!node.get_password(), Arc::new(key));
    /// ```
    pub fn get_password(&self) -> Arc<[u8; 16]> {
        self.password.clone()
    }

    /// posh port in node.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let node = Node::new(0, key.clone());
    /// // node.push_port(43196);
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
    /// ```ignore
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let node = Node::new(0, key.clone());
    /// // node.push_channel(0x4000);
    /// ```
    pub fn push_channel(&mut self, channel: u16) {
        if !self.channels.contains(&channel) {
            self.channels.push(channel);
        }
    }
}

/// node table.
pub struct Nodes {
    map: RwLock<HashMap<Addr, Node>>,
    bonds: RwLock<HashMap<String, HashSet<Addr>>>,
}

impl Nodes {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::with_capacity(capacity())),
            bonds: RwLock::new(HashMap::with_capacity(capacity())),
        }
    }

    /// get users name and address.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let node = Nodes::new();
    /// assert_eq!(!node.get_users().len(), 0);
    /// ```
    pub async fn get_users(&self) -> Vec<(String, Vec<SocketAddr>)> {
        self.bonds
            .read()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().map(|v| *v.clone()).collect()))
            .collect()
    }

    /// get node from name.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let node = Nodes::new();
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    ///
    /// node.insert(&addr, "test", key);
    ///
    /// assert!(!node.get_node("test").is_some());
    /// ```
    pub async fn get_nodes(&self, u: &str) -> Vec<Node> {
        let bounds = self.bonds.read().await;
        let map = self.map.read().await;
        let addrs = match bounds.get(u) {
            None => return Vec::new(),
            Some(a) => a,
        };

        let mut nodes = Vec::with_capacity(addrs.len());
        for addr in addrs {
            if let Some(node) = map.get(addr) {
                nodes.push(node.clone());
            }
        }

        nodes
    }

    /// get password from address.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let node = Nodes::new();
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    ///
    /// node.insert(&addr, "test", key);
    ///
    /// assert!(!node.get_password(&addr).is_some());
    /// ```
    pub async fn get_password(&self, a: &Addr) -> Option<Arc<[u8; 16]>> {
        self.map.read().await.get(a).map(|n| n.get_password())
    }

    /// insert node in node table.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let node = Nodes::new();
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    ///
    /// node.insert(&addr, "test", key);
    /// ```
    pub async fn insert(
        &self,
        a: &Addr,
        u: &str,
        p: [u8; 16],
    ) -> Option<Arc<[u8; 16]>> {
        let node = Node::new(u.to_string(), p);
        let pwd = node.get_password();
        let mut bonds = self.bonds.write().await;
        self.map.write().await.insert(a.clone(), node);

        bonds
            .entry(u.to_string())
            .or_insert_with(|| HashSet::with_capacity(5))
            .insert(a.clone());
        Some(pwd)
    }

    /// push port to node.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let node = Nodes::new();
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    ///
    /// node.insert(&addr, "test", key);
    ///
    /// node.push_port(&addr, 60000);
    /// ```
    pub async fn push_port(&self, a: &Addr, port: u16) -> Option<()> {
        self.map.write().await.get_mut(a)?.push_port(port);
        Some(())
    }

    /// push channel to node.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let node = Nodes::new();
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    ///
    /// node.insert(&addr, "test", key);
    ///
    /// node.push_channel(&addr, 0x4000);
    /// ```
    pub async fn push_channel(&self, a: &Addr, channel: u16) -> Option<()> {
        self.map.write().await.get_mut(a)?.push_channel(channel);
        Some(())
    }

    /// set lifetime to node.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let node = Nodes::new();
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    ///
    /// node.insert(&addr, "test", key);
    ///
    /// node.set_lifetime(&addr, 0);
    /// ```
    pub async fn set_lifetime(&self, a: &Addr, delay: u32) -> Option<()> {
        self.map.write().await.get_mut(a)?.set_lifetime(delay);
        Some(())
    }

    /// remove node from address.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let node = Nodes::new();
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    ///
    /// node.insert(&addr, "test", key);
    ///
    /// assert!(node.remove(&addr).is_some());
    /// ```
    pub async fn remove(&self, a: &Addr) -> Option<Node> {
        let mut bonds = self.bonds.write().await;
        let node = self.map.write().await.remove(a)?;
        let addrs = bonds.get_mut(&node.username)?;
        if addrs.is_empty() {
            bonds.remove(&node.username)?;
        } else {
            addrs.remove(a);
        }

        Some(node)
    }

    /// get node name bond address.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let node = Nodes::new();
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    ///
    /// node.insert(&addr, "test", key);
    ///
    /// assert_eq!(node.get_bond(&addr), Some(addr));
    /// ```
    pub async fn get_bond(&self, u: &str) -> Vec<Addr> {
        self.bonds
            .read()
            .await
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
    /// ```ignore
    /// let node = Nodes::new();
    /// assert_eq!(node.get_deaths().len(), 0);
    /// ```
    pub async fn get_deaths(&self) -> Vec<Addr> {
        self.map
            .read()
            .await
            .iter()
            .filter(|(_, v)| v.is_death())
            .map(|(k, _)| k.clone())
            .collect::<Vec<Addr>>()
    }
}
