use crate::controls::Auth;
use tokio::sync::RwLock;
use anyhow::Result;
use tokio::time::{
    Duration,
    Instant,
    sleep
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

/// 端口标识
///
/// `[0]` 分组号
/// `[1]` 端口号
#[derive(Hash, Eq)]
struct UniquePort(u32, u16);

/// 频道标识
///
/// `[0]` 分组号
/// `[1]` 端口号
/// `[2]` 频道号
#[derive(Hash, Eq)]
struct UniqueChannel(Addr, u16);

/// 节点
pub struct Node {
    pub group: u32,
    delay: u64,
    clock: Instant,
    ports: Vec<u16>,
    channels: Vec<u16>,
    password: Arc<String>,
}

/// 状态管理
///
/// * `peers` 对等绑定表
/// * `nonces` 随机ID分配表
/// * `channels` 频道分配列表
/// * `groups` 分组端口分配表
/// * `allocs` 端口分配表
/// * `base` 基础信息表
pub struct State {
    nonces: RwLock<HashMap<Addr, (Arc<String>, Instant)>>,
    peers: RwLock<HashMap<Addr, HashMap<Addr, u16>>>,
    channels: RwLock<HashMap<UniqueChannel, Addr>>,
    allocs: RwLock<HashMap<UniquePort, Addr>>,
    groups: RwLock<HashMap<u32, (u16, u16)>>,
    base: RwLock<HashMap<Addr, Node>>,
}

impl State {
    /// 创建实例
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crate::state::*;
    /// use crate::controls::Auth;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// let password = Arc::new("panda".to_string());
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    /// };
    /// 
    /// state.insert(addr.clone(), &auth).await;
    /// assert_eq!(state.get_password(&addr).await, Some(password));
    /// ```
    #[rustfmt::skip]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            channels: RwLock::new(HashMap::with_capacity(1024)),
            nonces: RwLock::new(HashMap::with_capacity(1024)),
            allocs: RwLock::new(HashMap::with_capacity(1024)),
            groups: RwLock::new(HashMap::with_capacity(1024)),
            peers: RwLock::new(HashMap::with_capacity(1024)),
            base: RwLock::new(HashMap::with_capacity(1024)),
        })
    }

    /// 获取随机ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crate::state::*;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// 
    /// let one = state.get_nonce(&addr).await;
    /// let two = state.get_nonce(&addr).await;
    /// assert_eq!(one.as_ref(), two.as_ref());
    /// ```
    #[rustfmt::skip]
    pub async fn get_nonce(&self, a: &Addr) -> Arc<String> {
        if let Some((n, c)) = self.nonces.read().await.get(a) {
            if c.elapsed().as_secs() >= 3600 {
                return n.clone()   
            }
        }

        let nonce = Arc::new(rand_string());
        self.nonces.write().await.insert(a.clone(), (
            nonce.clone(),
            Instant::now()
        ));

        nonce
    }

    /// 获取密钥
    ///
    /// # Unit Test
    ///
    /// ```test(get_password)
    /// use crate::state::*;
    /// use crate::controls::Auth;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// let password = Arc::new("panda".to_string());
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    /// };
    /// 
    /// state.insert(addr.clone(), &auth).await;
    /// assert_eq!(state.get_password(&addr).await, Some(password));
    /// ```
    pub async fn get_password(&self, a: &Addr) -> Option<Arc<String>> {
        self.base.read().await.get(a).map(|n| {
            n.password.clone()
        })
    }

    /// 写入节点信息
    ///
    /// # Unit Test
    ///
    /// ```test(insert)
    /// use super::*;
    /// use crate::controls::Auth;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    /// };
    /// 
    /// state.insert(addr.clone(), &auth).await;
    /// let base = state.base.read().await;
    /// let result = base.get(&addr).unwrap();
    /// assert_eq!(result.password, Arc::new("panda".to_string()));
    /// assert_eq!(result.group, 0);
    /// ```
    #[rustfmt::skip]
    pub async fn insert(&self, a: Addr, auth: &Auth) {
        self.base.write().await.insert(a, Node {
            password: Arc::new(auth.password.clone()),
            clock: Instant::now(),
            channels: Vec::new(),
            ports: Vec::new(),
            group: auth.group,
            delay: 600,
        });

        self.groups
            .write()
            .await
            .entry(auth.group)
            .or_insert_with(|| (1, 49152));
    }

     /// 绑定对端端口
    ///
    /// # Unit Test
    ///
    /// ```test(bind_peer)
    /// use super::*;
    /// use crate::controls::Auth;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// let peer_addr = Arc::new("127.0.0.1:49151".parse::<SocketAddr>().unwrap());
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0
    /// };
    /// 
    /// state.insert(addr.clone(), &auth).await;
    /// state.insert(peer_addr.clone(), &auth).await;
    /// assert_eq!(state.alloc_port(addr.clone()).await, Some(49152));
    /// assert_eq!(state.alloc_port(peer_addr.clone()).await, Some(49153));
    /// assert_eq!(state.bind_peer(&addr, 49153).await, true);
    /// assert_eq!(state.bind_peer(&peer_addr, 49152).await, true);
    /// ```
    pub async fn bind_peer(&self, a: &Addr, port: u16) -> bool {
        let p = match self.reflect_from_port(a, port).await {
            Some(a) => a,
            None => return false,
        };

        self.peers
            .write()
            .await
            .entry(p)
            .or_insert_with(|| HashMap::new())
            .insert(a.clone(), port);
        true
    }

    /// 分配端口
    ///
    /// # Unit Test
    ///
    /// ```test(alloc_port)
    /// use super::*;
    /// use crate::controls::Auth;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// let peer_addr = Arc::new("127.0.0.1:49151".parse::<SocketAddr>().unwrap());
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0
    /// };
    /// 
    /// state.insert(addr.clone(), &auth).await;
    /// state.insert(peer_addr.clone(), &auth).await;
    /// assert_eq!(state.alloc_port(addr).await, Some(49152));
    /// assert_eq!(state.alloc_port(peer_addr).await, Some(49153));
    /// ```
    pub async fn alloc_port(&self, a: Addr) -> Option<u16> {
        let mut base = self.base.write().await;
        let node = match base.get_mut(&a) {
            Some(n) => n,
            _ => return None
        };

        let mut groups = self.groups.write().await;
        let port = match groups.get_mut(&node.group) {
            Some(p) => p,
            _ => return None
        };

        if node.ports.len() == 16383 {
            return None
        }

        let alloc = port.1.clone();
        node.ports.push(alloc);

        port.0 += 1;
        if port.1 == 65535 {
            port.1 = 49152;
        } else {
            port.1 += 1;
        }
        
        self.allocs.write().await.insert(
            UniquePort(node.group, alloc),
            a
        );

        Some(alloc)
    }
    
    /// 分配频道
    ///
    /// # Unit Test
    ///
    /// ```test(insert_channel)
    /// use super::*;
    /// use crate::controls::Auth;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// let peer_addr = Arc::new("127.0.0.1:49151".parse::<SocketAddr>().unwrap());
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0
    /// };
    /// 
    /// state.insert(addr.clone(), &auth).await;
    /// state.insert(peer_addr.clone(), &auth).await;
    /// assert_eq!(state.alloc_port(addr.clone()).await, Some(49152));
    /// assert_eq!(state.alloc_port(peer_addr).await, Some(49153));
    /// assert_eq!(state.insert_channel(addr, 49153, 0x4000).await, true);
    /// ```
    #[rustfmt::skip]
    pub async fn insert_channel(&self, a: Addr, p: u16, channel: u16) -> bool {
        assert!(channel >= 0x4000 && channel <= 0x4FFF);
        let mut base = self.base.write().await;
        let node = match base.get_mut(&a) {
            Some(n) => n,
            _ => return false,
        };

        let id = UniquePort(node.group, p);
        let addr = match self.allocs.read().await.get(&id) {
            Some(a) => a.clone(),
            _ => return false,
        };

        if node.channels.contains(&channel) {
            return false
        }

        node.channels.push(channel);
        self.channels.write().await.insert(
            UniqueChannel(a, channel),
            addr
        );

        true
    }

    /// 获取对等节点绑定本地端口
    ///
    /// # Unit Test
    ///
    /// ```test(reflect_from_peer)
    /// use super::*;
    /// use crate::controls::Auth;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// let peer_addr = Arc::new("127.0.0.1:49151".parse::<SocketAddr>().unwrap());
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0
    /// };
    /// 
    /// state.insert(addr.clone(), &auth).await;
    /// state.insert(peer_addr.clone(), &auth).await;
    /// assert_eq!(state.alloc_port(addr.clone()).await, Some(49152));
    /// assert_eq!(state.alloc_port(peer_addr.clone()).await, Some(49153));
    /// assert_eq!(state.bind_peer(&addr, 49153).await, true);
    /// assert_eq!(state.bind_peer(&peer_addr, 49152).await, true);
    /// assert_eq!(state.reflect_from_peer(&addr, &peer_addr).await, Some(49152));
    /// ```
    pub async fn reflect_from_peer(&self, a: &Addr, p: &Addr) -> Option<u16> {
        match self.peers.read().await.get(a) {
            Some(peer) => peer.get(p).copied(),
            None => None,
        }
    }
    
    /// 获取对等节点
    ///
    /// # Unit Test
    ///
    /// ```test(reflect_from_port)
    /// use crate::state::*;
    /// use crate::controls::Auth;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// let peer_addr = Arc::new("127.0.0.1:49151".parse::<SocketAddr>().unwrap());
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0
    /// };
    ///    
    /// state.insert(addr.clone(), &auth).await;
    /// state.insert(peer_addr.clone(), &auth).await;
    /// assert_eq!(state.alloc_port(addr.clone()).await, Some(49152));
    /// assert_eq!(state.alloc_port(peer_addr.clone()).await, Some(49153));
    /// assert_eq!(state.reflect_from_port(&addr, 49153).await, Some(peer_addr.clone()));
    /// assert_eq!(state.reflect_from_port(&peer_addr, 49152).await, Some(addr.clone()));
    /// assert_eq!(state.reflect_from_port(&addr, 49154).await, None);
    /// ```
    #[rustfmt::skip]
    pub async fn reflect_from_port(&self, a: &Addr, port: u16) -> Option<Addr> {
        assert!(port >= 49152);
        let group = match self.base.read().await.get(a) {
            Some(n) => n.group,
            _ => return None
        };

        self.allocs
            .read()
            .await
            .get(&UniquePort(group, port))
            .cloned()
    }
    
    /// 刷新节点生命周期
    ///
    /// # Unit Test
    ///
    /// ```test(refresh) 
    /// use crate::state::*;
    /// use crate::controls::Auth;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0
    /// };
    ///     
    /// state.insert(addr.clone(), &auth).await;
    /// assert_eq!(state.refresh(&addr, 600).await, true);
    /// assert_eq!(state.get_password(&addr).await.is_some(), true);
    /// assert_eq!(state.refresh(&addr, 0).await, true);
    /// assert_eq!(state.get_password(&addr).await.is_some(), false);
    /// ```
    #[rustfmt::skip]
    pub async fn refresh(&self, a: &Addr, delay: u32) -> bool {
        if delay == 0 {
            self.remove(a).await;
            return true;
        }
        
        self.base.write().await.get_mut(a).map(|n| {
            n.clock = Instant::now();
            n.delay = delay as u64;
        }).is_some()
    }
    
    /// 获取对等频道
    ///
    /// # Unit Test
    ///
    /// ```test(reflect_from_channel) 
    /// use crate::state::*;
    /// use crate::controls::Auth;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///  
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// let peer_addr = Arc::new("127.0.0.1:49151".parse::<SocketAddr>().unwrap());
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    /// };
    ///     
    /// state.insert(addr.clone(), &auth).await;
    /// state.insert(peer_addr.clone(), &auth).await;
    /// assert_eq!(state.alloc_port(addr.clone()).await, Some(49152));
    /// assert_eq!(state.alloc_port(peer_addr.clone()).await, Some(49153));
    /// state.insert_channel(addr.clone(), 49153, 0x4000).await;
    /// state.insert_channel(peer_addr.clone(), 49152, 0x4000).await;
    /// assert_eq!(state.reflect_from_channel(&addr, 0x4000).await, Some(peer_addr.clone()));
    /// assert_eq!(state.reflect_from_channel(&peer_addr, 0x4000).await, Some(addr));
    /// ```
    #[rustfmt::skip]
    pub async fn reflect_from_channel(&self, a: &Addr, channel: u16) -> Option<Addr> {
        assert!(channel >= 0x4000 && channel <= 0x4FFF);
        self.channels.read().await.get(
            &UniqueChannel(a.clone(), channel)
        ).cloned()
    }
    
    /// 删除节点
    ///
    /// # Unit Test
    ///
    /// ```test(remove)
    /// use crate::state::*;
    /// use crate::controls::Auth;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new("127.0.0.1:49152".parse::<SocketAddr>().unwrap());
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0
    /// };
    ///     
    /// state.insert(addr.clone(), &auth).await;
    /// assert_eq!(state.get_password(&addr).await.is_some(), true);
    /// state.remove(&addr).await;
    /// assert_eq!(state.get_password(&addr).await.is_some(), false);
    /// ```
    #[rustfmt::skip]
    pub async fn remove(&self, a: &Addr) {
        let mut allocs = self.allocs.write().await;
        let mut channels = self.channels.write().await;
        let mut groups = self.groups.write().await;
        let node = match self.base.write().await.remove(a) {
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
        
        self.peers
            .write()
            .await
            .remove(a);
        self.nonces
            .write()
            .await
            .remove(a);
    }
    
    /// 启动实例
    ///
    /// 定时扫描内部列表
    /// 扫描间隔为60秒
    /// 
    /// # Example
    ///
    /// ```no_run
    /// use crate::state::*;
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     State::new(None)
    ///         .run()
    ///         .await
    ///         .unwrap();
    /// }
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

    /// 清理失效节点
    ///
    /// 删除所有失效节点以及其关联信息，当删除完成之后，
    /// 所有分配的频道和端口也将失效
    #[rustfmt::skip]
    async fn clear(&self) {
        let fails = self.base
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

/// 生成随机ID
///
/// # Unit Test
///
/// ```test(rand_string)
/// let rands = super::rand_string();
/// assert_eq!(rands.len(), 16);
/// ```
fn rand_string() -> String {
    let mut rng = thread_rng();
    let r = std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect::<String>();
    r.to_lowercase()
}

/// 是否超时
///
/// # Unit Test
///
/// ```test(is_timeout)
/// use tokio::time::Instant;
/// use std::sync::Arc;
/// 
/// assert_eq!(super::is_timeout(&super::Node {
///     password: Arc::new("".to_string()),
///     clock: Instant::now(),
///     channels: vec![],
///     ports: vec![],
///     delay: 600,
///     group: 0
/// }), false);
/// ```
fn is_timeout(n: &Node) -> bool {
    n.clock.elapsed().as_secs() >= n.delay
}