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
    net::SocketAddr,
    sync::Arc,
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
    pub secret: Arc<[u8; 16]>,
    pub username: String,
    pub password: String,
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
    pub fn new(username: String, secret: [u8; 16], password: String) -> Self {
        Self {
            channels: Vec::with_capacity(5),
            ports: Vec::with_capacity(10),
            secret: Arc::new(secret),
            timer: Instant::now(),
            lifetime: 600,
            username,
            password,
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

    /// get node the secret.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let node = Node::new(0, key.clone());
    /// assert_eq!(!node.get_secret(), Arc::new(key));
    /// ```
    pub fn get_secret(&self) -> Arc<[u8; 16]> {
        self.secret.clone()
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
    addrs: RwLock<HashMap<String, HashSet<Addr>>>,
}

impl Nodes {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::with_capacity(capacity())),
            addrs: RwLock::new(HashMap::with_capacity(capacity())),
        }
    }

    /// get users name and address.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let node = Nodes::new();
    /// assert_eq!(!node.get_users(0, 10).len(), 0);
    /// ```
    pub async fn get_users(
        &self,
        skip: usize,
        limit: usize,
    ) -> Vec<(String, Vec<SocketAddr>)> {
        self.addrs
            .read()
            .await
            .iter()
            .skip(skip)
            .take(limit)
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
    pub async fn get_node(&self, a: &Addr) -> Option<Node> {
        self.map.read().await.get(a).cloned()
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
    pub async fn get_secret(&self, a: &Addr) -> Option<Arc<[u8; 16]>> {
        self.map.read().await.get(a).map(|n| n.get_secret())
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
        s: [u8; 16],
        p: String,
    ) -> Option<Arc<[u8; 16]>> {
        let node = Node::new(u.to_string(), s, p);
        let pwd = node.get_secret();
        let mut addrs = self.addrs.write().await;
        self.map.write().await.insert(a.clone(), node);

        addrs
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
        let mut addrs_map = self.addrs.write().await;
        let node = self.map.write().await.remove(a)?;
        let addrs = addrs_map.get_mut(&node.username)?;
        if addrs.len() == 1 {
            addrs_map.remove(&node.username)?;
        } else {
            addrs.remove(a);
        }

        Some(node)
    }

    /// get node name bound address.
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
    /// assert_eq!(node.get_bound(&addr), Some(addr));
    /// ```
    pub async fn get_addrs(&self, u: &str) -> Vec<Addr> {
        self.addrs
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
