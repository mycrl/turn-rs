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
        let channel = self.channels.write().await
            .entry(c)
            .or_insert_with(|| Channel::new(a));

        if channel.is_half() {
            channel.up(a);
        } else {
            if channel.includes(a) {
                return Err(anyhow!("bond already exists"))   
            } else {
                channel.refresh();
            }
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
        let mut ports = self.ports.write().await;
        let mut channels = self.channels.write().await;
        let mut channel_bonds = self.channel_bonds.write().await;
        let mut nodes = self.nodes.write().await;
        let node = match nodes.remove(a) {
            Some(n) => n,
            None => return
        };

        for port in node.ports {
            ports.remove(&port);
        }

        let mut half = Vec::with_capacity(5);
        for channel in node.channels {
            if let Some(cs) = channels.remove(&channel) {
                for (_, addr) in cs.bond.iter().enumerate() {
                    if let Some(i) = addr {
                        channel_bonds.remove(&(i.clone(), channel));
                        if i != a {
                            half.push((i.clone(), channel));
                        }
                    }
                }
            }
        }

        for (half_addr, channel) in half {
            if let Some(node) = nodes.get_mut(&half_addr) {
                if let Some(index) = first_index(&node.channels, channel) {
                    node.channels.swap_remove(index);
                }
            }
        }
    }
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
