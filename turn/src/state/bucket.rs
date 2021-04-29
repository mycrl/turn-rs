use random_port::RandomPort;
use tokio::sync::RwLock;
use super::{
    channel::Channel,
    Addr
};

use anyhow::{
    Result,
    anyhow
};

use tokio::time::{
    Duration,
    Instant,
    sleep
};

use crate::broker::{
    response::Auth,
    Broker
};

use rand::{
    distributions::Alphanumeric, 
    thread_rng, 
    Rng
};

use std::{
    collections::HashMap,
    net::SocketAddr,
    cmp::PartialEq,
    sync::Arc,
};

/// client session.
pub struct Node {
    timer: Instant,
    lifetime: u64,
    ports: Vec<u16>,
    channels: Vec<u16>,
}

impl Node {
    pub fn pop_channel(&mut self, c: u16) -> Option<()> {
        let index = Self::first_index(&self.channels, c)?;
        self.channels.swap_remove(index);
        Some(())
    }

    pub fn pop_port(&mut self, c: u16) -> Option<()> {
        let index = Self::first_index(&self.ports, c)?;
        self.ports.swap_remove(index);
        Some(())
    }

    /// find item first index in vector.
    fn first_index(raw: &Vec<u16>, value: u16) -> Option<usize> {
        for (index, item) in raw.iter().enumerate() {
            if item == &value {
                return Some(index)
            }
        }

        None
    }
}

//{
//        "nodes": {
//            "127.0.0.1:8080": {
//                "timer": 100,
//                "lifetime": 100,
//                "ports": [
//                    49152
//                ]
//            }
//        },
//        "ports": {
//            "49152": "127.0.0.1:8080"
//        },
//        "channels": {
//            "0x4000": {
//                "timer": 100,
//                "bond": [
//                    "127.0.0.1:8080",
//                    "127.0.0.1:8081"
//                ]
//            }
//        },
//        "channel_bonds": {
//            "127.0.0.1:8080_0x4000": "127.0.0.1:8081",
//            "127.0.0.1:8081_0x4000": "127.0.0.1:8080"
//        }
//    }

pub struct Bucket {
    port: RwLock<RandomPort>,
    nodes: RwLock<HashMap<Addr, Node>>,
    ports: RwLock<HashMap<u16, Addr>>,
    channels: RwLock<HashMap<u16, Channel>>,
    channel_bonds: RwLock<HashMap<(Addr, u16), Addr>>,
}

impl Bucket {
    /// ```no_run
    /// use tokio::time::Instant;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let node = Node {
    ///     timer: Instant::now(),
    ///     lifetime: 600,
    ///     ports: Vec::new(),
    ///     channels: Vec::new()
    /// };
    ///
    /// let bucket = Bucket::new();
    /// bucket.insert_node(Arc::new(addr), node).unwrap();
    /// ```
    pub async fn insert_node(&self, a: Addr, n: Node) -> Result<()> {
        self.nodes.write().await.insert(a, n)
            .ok_or_else(|| anyhow!("insert node failed"))
            .map(|_| ())
    }
   
    /// ```no_run
    /// use tokio::time::Instant;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let node = Node {
    ///     timer: Instant::now(),
    ///     lifetime: 600,
    ///     ports: Vec::new(),
    ///     channels: Vec::new()
    /// };
    ///
    /// let bucket = Bucket::new();
    /// bucket.insert_node(Arc::new(addr), node).unwrap();
    /// // bucket.alloc_port(&Arc::new(addr)).unwrap();
    /// ```
    pub async fn alloc_port(&self, a: &Addr) -> Option<u16> {
        let mut nodes = self.nodes.write().await;
        let node = match nodes.get_mut(a) {
            Some(n) => n,
            None => return None
        };

        let port = match self.port.write().await.alloc(None) {
            Some(p) => p,
            None => return None
        };

        self.ports
            .write()
            .await
            .insert(port, a.clone());
        node.ports.push(port);
        Some(port)
    }
    
    /// ```no_run
    /// use tokio::time::Instant;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let node = Node {
    ///     timer: Instant::now(),
    ///     lifetime: 600,
    ///     ports: Vec::new(),
    ///     channels: Vec::new()
    /// };
    ///
    /// let bucket = Bucket::new();
    /// bucket.insert_node(Arc::new(addr), node).unwrap();
    /// let port = bucket.alloc_port(&Arc::new(addr)).unwrap();
    /// // bucket.bind_port(port).unwrap();
    /// ```
    pub async fn bind_port(&self, port: u16) -> Result<()> {
        self.ports
            .read()
            .await
            .contains_key(&port)
            .then(|| ())
            .ok_or_else(|| anyhow!("port not found"))
    }

    /// ```no_run
    /// use tokio::time::Instant;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let node = Node {
    ///     timer: Instant::now(),
    ///     lifetime: 600,
    ///     ports: Vec::new(),
    ///     channels: Vec::new()
    /// };
    ///
    /// let bucket = Bucket::new();
    /// bucket.insert_node(Arc::new(addr), node).unwrap();
    /// // bucket.bind_channel(49152, 0x4000).unwrap();
    /// ```
    pub async fn bind_channel(&self, a: &Addr, p: u16, c: u16) -> Result<()> {
        let mut channels = self.channels.write().await;
        let channel = channels.entry(c).or_insert_with(|| Channel::new(a));
        let is_half = channel.is_half();

        if is_half {
            channel.up(a);
        }

        if !is_half && channel.includes(a) {
            return Err(anyhow!("bond already exists"))   
        }

        if !is_half {
            channel.refresh();
        }

        let source = match self.ports.read().await.get(&p) {
            None => return Err(anyhow!("port not found")),
            Some(a) => a.clone()
        };

        if let Some(node) = self.nodes.write().await.get_mut(a) {
            if !node.channels.contains(&c) {
                node.channels.push(c)
            }
        }

        self.channel_bonds
            .write()
            .await
            .entry((a.clone(), c))
            .or_insert_with(|| source);
        Ok(())
    }

    /// ```no_run
    /// use tokio::time::Instant;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let node = Node {
    ///     timer: Instant::now(),
    ///     lifetime: 600,
    ///     ports: Vec::new(),
    ///     channels: Vec::new()
    /// };
    ///
    /// let bucket = Bucket::new();
    /// bucket.insert_node(Arc::new(addr), node).unwrap();
    /// // bucket.bind_channel(49152, 0x4000).unwrap();
    /// ```
    pub async fn refresh(&self, a: &Addr, delay: u32) {

    }

    /// ```no_run
    /// use tokio::time::Instant;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let node = Node {
    ///     timer: Instant::now(),
    ///     lifetime: 600,
    ///     ports: Vec::new(),
    ///     channels: Vec::new()
    /// };
    ///
    /// let bucket = Bucket::new();
    /// bucket.insert_node(Arc::new(addr), node).unwrap();
    /// // bucket.bind_channel(49152, 0x4000).unwrap();
    /// ```
    pub async fn remove(&self, a: &Addr) {
        let mut port = self.port.write().await;
        let mut nodes = self.nodes.write().await;
        let mut ports = self.ports.write().await;
        let mut channels = self.channels.write().await;
        let mut channel_bonds = self.channel_bonds.write().await;
        let node = match nodes.remove(a) {
            Some(n) => n,
            None => return
        };

        for p in node.ports {
            if ports.remove(&p).is_some() {
                port.restore(p);
            }
        }

        for num in node.channels {
            if let Some(channel) = channels.remove(&num) {
                for addr in channel.bond.iter() {
                    if let Some(i) = addr {
                        channel_bonds.remove(&(i.clone(), num));
                        if i != a {
                            if let Some(n) = nodes.get_mut(i) {
                                n.pop_channel(num);
                            }
                        }
                    }
                }
            }
        }
    }
}
