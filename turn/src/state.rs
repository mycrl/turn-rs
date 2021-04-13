use tokio::sync::RwLock;
use anyhow::Result;
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

type Addr = Arc<SocketAddr>;

/// port mark.
#[derive(Hash, Eq)]
struct UniquePort(
    /// group number
    u32, 
    /// port number
    u16
);

/// channel mark.
#[derive(Hash, Eq)]
struct UniqueChannel(
    /// client address
    Addr, 
    /// channel number
    u16
);

/// client session.
pub struct Node {
    /// the group where the node is located.
    pub group: u32,
    /// session timeout.
    pub delay: u64,
    /// record refresh time.
    pub clock: Instant,
    /// list of ports allocated for the current session.
    pub ports: Vec<u16>,
    /// list of channels allocated for the current session.
    pub channels: Vec<u16>,
    /// the key of the current session.
    pub password: Arc<String>,
}

/// session state manager.
pub struct State {
    broker: Arc<Broker>,
    pub base_table: RwLock<HashMap<Addr, Node>>,
    /// assign a random ID with timeout to each user.
    nonce_table: RwLock<HashMap<Addr, (Arc<String>, Instant)>>,
    /// record the port binding relationship between the session and the peer.
    peer_table: RwLock<HashMap<Addr, HashMap<Addr, u16>>>,
    /// record the binding relationship between channels and addresses in the group.
    channel_table: RwLock<HashMap<UniqueChannel, Addr>>,
    /// record the binding relationship between port and address in the group.
    port_table: RwLock<HashMap<UniquePort, Addr>>,
    /// record the reference count and offset of the port in the group.
    group_port_rc: RwLock<HashMap<u32, (u16, u16)>>,
}

/// simpler to generate structure.
macro_rules! builder_state {
    ([$($label:ident),*], $broker:tt) => {
        State {
            $($label: RwLock::new(HashMap::with_capacity(1024)),)*
            $broker
        }
    }
}

impl State {
    /// # Example
    ///
    /// ```no_run
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// // State::new(t)
    /// ```
    #[rustfmt::skip]
    pub fn new(broker: Arc<Broker>) -> Arc<Self> {
        Arc::new(builder_state!([
            group_port_rc,
            channel_table,
            nonce_table,
            port_table,
            peer_table,
            base_table
        ], broker))
    }

    /// get node password.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync:::Arc;
    /// 
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// let state = State::new(t);
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    /// // state.get_password(&addr, "panda")
    /// ```
    pub async fn get_password(&self, a: &Addr, u: &str) -> Option<Arc<String>> {
        if let Some(auth) = self.base_table.read().await.get(a) {
             return Some(auth.password.clone())
         }
        
         let auth = match self.broker.auth(a.as_ref(), u).await {
             Err(_) => return None,
             Ok(a) => a
         };
        
         self.insert(a.clone(), &auth).await;
         Some(Arc::new(auth.password))
    }
    
    /// get nonce string.
    /// According to the RFC, the expiration time is 1 hour.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync:::Arc;
    /// 
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// let state = State::new(t);
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    /// // state.get_nonce(&addr)
    /// ```
    #[rustfmt::skip]
    pub async fn get_nonce(&self, a: &Addr) -> Arc<String> {
        if let Some((n, c)) = self.nonce_table.read().await.get(a) {
            if c.elapsed().as_secs() >= 3600 {
                return n.clone()   
            }
        }

        let nonce = Arc::new(rand_string());
        self.nonce_table.write().await.insert(a.clone(), (
            nonce.clone(),
            Instant::now()
        ));

        nonce
    }

    /// insert node info.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync:::Arc;
    /// 
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// let state = State::new(t);
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    /// // state.insert(addr, Auth {
    /// //     password: "panda".to_string(),
    /// //     group: 0
    /// // })
    /// ```
    #[rustfmt::skip]
    pub async fn insert(&self, a: Addr, auth: &Auth) {
        self.base_table.write().await.insert(a, Node {
            password: Arc::new(auth.password.clone()),
            clock: Instant::now(),
            channels: Vec::new(),
            ports: Vec::new(),
            group: auth.group,
            delay: 600,
        });

        self.group_port_rc
            .write()
            .await
            .entry(auth.group)
            .or_insert_with(|| (1, 49152));
    }

    /// establish a binding relationship 
    /// through its own address and peer port.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync:::Arc;
    /// 
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// let state = State::new(t);
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    /// // state.bind_peer(&addr, 8081)
    /// ```
    pub async fn bind_peer(&self, a: &Addr, port: u16) -> bool {
        let peer = match self.reflect_from_port(a, port).await {
            Some(a) => a,
            None => return false,
        };

        self.peer_table
            .write()
            .await
            .entry(peer)
            .or_insert_with(HashMap::new)
            .insert(a.clone(), port);
        true
    }

    /// allocate port to node.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync:::Arc;
    /// 
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// let state = State::new(t);
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    /// // state.alloc_port(addr)
    /// ```
    pub async fn alloc_port(&self, a: Addr) -> Option<u16> {
        let mut base = self.base_table.write().await;
        let node = match base.get_mut(&a) {
            Some(n) => n,
            _ => return None
        };

        let mut groups = self.group_port_rc.write().await;
        let port = match groups.get_mut(&node.group) {
            Some(p) => p,
            _ => return None
        };

        let alloc = port.1;
        node.ports.push(alloc);

        port.0 += 1;
        if port.1 == 65535 {
            port.1 = 49152;
        } else {
            port.1 += 1;
        }
        
        self.port_table.write().await.insert(
            UniquePort(node.group, alloc),
            a
        );

        Some(alloc)
    }
    
    /// allocate channel to node.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync:::Arc;
    /// 
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// let state = State::new(t);
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    /// // state.insert_channel(addr, 8081, 0)
    /// ```
    #[rustfmt::skip]
    pub async fn insert_channel(&self, a: Addr, p: u16, channel: u16) -> bool {
        assert!((0x4000..=0x4FFF).contains(&channel));
        let mut base = self.base_table.write().await;
        let node = match base.get_mut(&a) {
            Some(n) => n,
            _ => return false,
        };

        let id = UniquePort(node.group, p);
        let addr = match self.port_table.read().await.get(&id) {
            Some(a) => a.clone(),
            _ => return false,
        };

        if node.channels.contains(&channel) {
            return false
        }

        node.channels.push(channel);
        self.channel_table
            .write()
            .await
            .insert(
                UniqueChannel(a, channel),
                addr
            );

        true
    }

    /// get the local port bound to the peer node.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync:::Arc;
    /// 
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// let state = State::new(t);
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    /// let peer_addr = Arc::new("127.0.0.1:8081".parse::<SocketAddr>().unwrap());
    /// // state.reflect_from_peer(&addr, &peer_addr)
    /// ```
    pub async fn reflect_from_peer(&self, a: &Addr, p: &Addr) -> Option<u16> {
        match self.peer_table.read().await.get(a) {
            Some(peer) => peer.get(p).copied(),
            None => None,
        }
    }
    
    /// obtain the peer address according to 
    /// its own address and peer port.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync:::Arc;
    /// 
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// let state = State::new(t);
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    /// // state.reflect_from_port(&addr, 8081)
    /// ```
    #[rustfmt::skip]
    pub async fn reflect_from_port(&self, a: &Addr, port: u16) -> Option<Addr> {
        assert!(port >= 49152);
        let group = match self.base_table.read().await.get(a) {
            Some(n) => n.group,
            _ => return None
        };

        self.port_table
            .read()
            .await
            .get(&UniquePort(group, port))
            .cloned()
    }
    
    /// refresh node lifetime.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync:::Arc;
    /// 
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// let state = State::new(t);
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    /// // state.refresh(&addr, 0)
    /// ```
    #[rustfmt::skip]
    pub async fn refresh(&self, a: &Addr, delay: u32) -> bool {
        if delay == 0 {
            self.remove(a).await;
            return true;
        }
        
        self.base_table
            .write()
            .await
            .get_mut(a)
            .map(|n| {
                n.clock = Instant::now();
                n.delay = delay as u64;
            })
            .is_some()
    }
    
    /// get peer channel.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync:::Arc;
    /// 
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// let state = State::new(t);
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    /// // state.reflect_from_channel(&addr, 0)
    /// ```
    #[rustfmt::skip]
    pub async fn reflect_from_channel(&self, a: &Addr, channel: u16) -> Option<Addr> {
        assert!((0x4000..=0x4FFF).contains(&channel));
        self.channel_table.read().await.get(
            &UniqueChannel(a.clone(), channel)
        ).cloned()
    }
    
    /// remove node.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync:::Arc;
    /// 
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// let state = State::new(t);
    /// let addr = Arc::new("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    /// // state.remove(&addr)
    /// ```
    #[rustfmt::skip]
    pub async fn remove(&self, a: &Addr) {
        let mut allocs = self.port_table.write().await;
        let mut channels = self.channel_table.write().await;
        let mut groups = self.group_port_rc.write().await;
        let node = match self.base_table.write().await.remove(a) {
            Some(a) => a,
            None => return,
        };

        let port = match groups.get_mut(&node.group) {
            Some(p) => p,
            _ => return,
        };
        
        for port in node.ports {
            allocs.remove(&UniquePort(
                node.group,
                port
            ));
        }

        for channel in node.channels {
            channels.remove(&UniqueChannel(
                a.clone(),
                channel
            ));
        }

        if port.0 <= 1 {
            groups.remove(&node.group);
        } else {
            port.0 -= 1;
        }
        
        self.peer_table
            .write()
            .await
            .remove(a);
        self.nonce_table
            .write()
            .await
            .remove(a);
    }
    
    /// start state.
    ///
    /// scan the internal list regularly, 
    /// the scan interval is 60 second.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let c = config::Conf::new()?;
    /// let t = broker::Broker::new(&c.controls).await?;
    /// 
    /// // State::new(t).run().await?;
    /// ```
    #[rustfmt::skip]
    pub async fn run(self: Arc<Self>) -> Result<()> {
        let delay = Duration::from_secs(60);
        tokio::spawn(async move { 
            loop {
                sleep(delay).await;
                self.clear().await;
            }
        }).await?;
        Ok(())
    }

    /// clear all invalid node.
    #[rustfmt::skip]
    async fn clear(&self) {
        let fails = self.base_table
            .read()
            .await
            .iter()
            .filter(|(_, v)| is_timeout(v))
            .map(|(k, _)| k.clone())
            .collect::<Vec<Addr>>();
        for a in fails {
            self.remove(&a).await;
        }
    }
}

impl PartialEq for UniquePort {
    fn eq(&self, o: &Self) -> bool {
        self.0 == o.0 && self.1 == o.1
    }
}

impl PartialEq for UniqueChannel {
    fn eq(&self, o: &Self) -> bool {
        self.0 == o.0 && self.1 == o.1
    }
}

fn rand_string() -> String {
    let mut rng = thread_rng();
    let r = std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect::<String>();
    r.to_lowercase()
}

fn is_timeout(n: &Node) -> bool {
    n.clock.elapsed().as_secs() >= n.delay
}
