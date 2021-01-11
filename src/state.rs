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
    convert::From,
    sync::Arc,
};

type Addr = Arc<SocketAddr>;

/// 标识
///
/// `[0]` 分组号
/// `[1]` 端口或者频道
#[derive(Hash, Eq)]
struct UniqueId(u32, u16);

/// 节点
///
/// `delay` 超时时间
/// `channel` 频道号
/// `clock` 内部刷新时钟
struct Node {
    delay: u64,
    channel: u16,
    clock: Instant,
}

/// 状态管理
///
/// * `channels` 频道分配列表
/// * `allocs` 端口分配表
/// * `base` 基础信息表
/// * `nodes` 节点列表
pub struct State {
    channels: RwLock<HashMap<UniqueId, [Option<Addr>; 2]>>,
    nonces: RwLock<HashMap<Addr, (Arc<String>, Instant)>>,
    allocs: RwLock<HashMap<UniqueId, Addr>>,
    base: RwLock<HashMap<Addr, Arc<Auth>>>,
    nodes: RwLock<HashMap<Addr, Node>>,
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
    /// let addr = Arc::new(
    ///     "127.0.0.1:49152"
    ///         .parse::<SocketAddr>()
    ///         .unwrap()
    /// );
    /// 
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    ///     port: 49152
    /// };
    /// 
    /// let result = state.insert(addr.clone(), auth).await;
    /// assert_eq!(state.get(&addr).await, Some(result));
    /// ```
    #[rustfmt::skip]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            channels: RwLock::new(HashMap::with_capacity(1024)),
            nonces: RwLock::new(HashMap::with_capacity(1024)),
            allocs: RwLock::new(HashMap::with_capacity(1024)),
            nodes: RwLock::new(HashMap::with_capacity(1024)),
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
    /// let addr = Arc::new(
    ///     "127.0.0.1:49152"
    ///         .parse::<SocketAddr>()
    ///         .unwrap()
    /// );
    /// 
    /// let one = state.get_nonce(&addr).await;
    /// let two = state.get_nonce(addr).await;
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
    /// let addr = Arc::new(
    ///     "127.0.0.1:49152"
    ///         .parse::<SocketAddr>()
    ///         .unwrap()
    /// );
    /// 
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    ///     port: 49152
    /// };
    /// 
    /// let result = state.insert(addr, auth).await;
    /// assert_eq!(result.password, "panda");
    /// assert_eq!(result.group, 0);
    /// assert_eq!(result.port, 49152);
    /// ```
    #[rustfmt::skip]
    pub async fn insert(&self, a: Addr, auth: Auth) -> Arc<Auth> {
        let inner = Arc::new(auth);
        self.allocs.write().await.insert(UniqueId::from(&inner), a.clone());
        self.nodes.write().await.insert(a.clone(), Node::default());
        self.base.write().await.insert(a, inner.clone());
        inner
    }
    
    /// 创建频道
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
    /// let addr = Arc::new(
    ///     "127.0.0.1:49152"
    ///         .parse::<SocketAddr>()
    ///         .unwrap()
    /// );
    /// 
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    ///     port: 49152
    /// };
    ///     
    /// assert_eq!(state.insert_channel(addr.clone(), 0x4000).await, false);
    /// state.insert(addr.clone(), auth).await;
    /// assert_eq!(state.insert_channel(addr, 0x4000).await, true);
    /// ```
    #[rustfmt::skip]
    pub async fn insert_channel(&self, a: Addr, channel: u16) -> bool {
        assert!(channel >= 0x4000 && channel <= 0x4FFF);
        let mut channels = self.channels.write().await;
        let group = match self.base.read().await.get(&a) {
            Some(n) => n.group,
            _ => return false,
        };

        if let Some(n) = self.nodes.write().await.get_mut(&a) {
            n.channel = channel;
        }
        
        let id = UniqueId(group, channel);
        if let Some(p) = channels.get_mut(&id) {
            p[1] = Some(a);
            return true;
        }
        
        channels.insert(
            id, 
            [Some(a), None]
        );

        true
    }
    
    /// 获取基本信息
    ///
    /// # Unit Test
    ///
    /// ```test(get)
    /// use crate::state::*;
    /// use crate::controls::Auth;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// let state = State::new();
    /// let addr = Arc::new(
    ///     "127.0.0.1:49152"
    ///         .parse::<SocketAddr>()
    ///         .unwrap()
    /// );
    /// 
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    ///     port: 49152
    /// };
    ///     
    /// assert_eq!(state.get(&addr).await.is_some(), false);
    /// state.insert(addr.clone(), auth).await;
    /// assert_eq!(state.get(&addr).await.is_some(), true);
    /// ```
    pub async fn get(&self, a: &Addr) -> Option<Arc<Auth>> {
        self.base.read().await.get(a).cloned()
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
    ///     group: 0,
    ///     port: 49152
    /// };
    /// 
    /// let peer_auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    ///     port: 49153
    /// };
    ///    
    /// state.insert(addr.clone(), auth).await;
    /// state.insert(peer_addr.clone(), peer_auth).await;
    /// assert_eq!(state.reflect_from_port(&addr, 49153).await, Some(peer_addr.clone()));
    /// assert_eq!(state.reflect_from_port(&peer_addr, 49152).await, Some(addr.clone()));
    /// assert_eq!(state.reflect_from_port(&addr, 49154).await, None);
    /// ```
    #[rustfmt::skip]
    pub async fn reflect_from_port(&self, a: &Addr, port: u16) -> Option<Addr> {
        assert!(port >= 49152);
        let group = match self.base.read().await.get(a) {
            Some(n) if n.port != port => n.group,
            _ => return None
        };

        self.allocs
            .read()
            .await
            .get(&UniqueId(group, port))
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
    /// let addr = Arc::new(
    ///     "127.0.0.1:49152"
    ///         .parse::<SocketAddr>()
    ///         .unwrap()
    /// );
    /// 
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    ///     port: 49152
    /// };
    ///     
    /// state.insert(addr.clone(), auth).await;
    /// assert_eq!(state.refresh(&addr, 600).await, true);
    /// assert_eq!(state.get(&addr).await.is_some(), true);
    /// assert_eq!(state.refresh(&addr, 0).await, true);
    /// assert_eq!(state.get(&addr).await.is_some(), false);
    /// ```
    #[rustfmt::skip]
    pub async fn refresh(&self, a: &Addr, delay: u32) -> bool {
        if delay == 0 {
            self.remove(a).await;
            return true;
        }
        
        self.nodes.write().await.get_mut(a).map(|n| {
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
    ///     port: 49152
    /// };
    /// 
    /// let peer_auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    ///     port: 49153
    /// };
    ///     
    /// state.insert(addr.clone(), auth).await;
    /// state.insert(peer_addr.clone(), peer_auth).await;
    /// state.insert_channel(addr.clone(), 0x4000).await;
    /// state.insert_channel(peer_addr.clone(), 0x4000).await;
    /// assert_eq!(state.reflect_from_channel(&addr, 0x4000).await, Some(peer_addr.clone()));
    /// assert_eq!(state.reflect_from_channel(&peer_addr, 0x4000).await, Some(addr));
    /// ```
    #[rustfmt::skip]
    pub async fn reflect_from_channel(&self, a: &Addr, channel: u16) -> Option<Addr> {
        assert!(channel >= 0x4000 && channel <= 0x4FFF);
        let group = match self.base.read().await.get(a) {
            Some(n) => n.group,
            None => return None
        };
        
        let id = UniqueId(group, channel);
        if let Some(x) = self.channels.read().await.get(&id) {
            for v in x.iter() {
                if v.is_some() && *v != Some(a.clone()) {
                    return v.clone()
                }
            }
        }
        
        None
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
    /// let addr = Arc::new(
    ///     "127.0.0.1:49152"
    ///         .parse::<SocketAddr>()
    ///         .unwrap()
    /// );
    /// 
    /// let auth = Auth {
    ///     password: "panda".to_string(),
    ///     group: 0,
    ///     port: 49152
    /// };
    ///     
    /// state.insert(addr.clone(), auth).await;
    /// assert_eq!(state.get(&addr).await.is_some(), true);
    /// state.remove(&addr).await;
    /// assert_eq!(state.get(&addr).await.is_some(), false);
    /// ```
    #[rustfmt::skip]
    pub async fn remove(&self, a: &Addr) {
        let auth = match self.base.write().await.remove(a) {
            Some(a) => a,
            None => return,
        };
        
        self.nonces.write().await.remove(a);
        self.allocs.write().await.remove(&UniqueId(
            auth.group,
            auth.port
        ));
        
        let node = match self.nodes.write().await.remove(a) {
            Some(n) => n,
            None => return,
        };
        
        self.channels.write().await.remove(&UniqueId(
            auth.group, 
            node.channel
        ));
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
        let fails = self.nodes
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

impl Default for Node {
    fn default() -> Self {
        Self {
            clock: Instant::now(),
            channel: 0,
            delay: 600,
        }
    }
}

impl PartialEq for UniqueId {
    fn eq(&self, o: &Self) -> bool {
        self.0 == o.0 && self.1 == o.1
    }
}

impl From<&Arc<Auth>> for UniqueId {
    fn from(n: &Arc<Auth>) -> Self {
        Self(n.group, n.port)
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
/// 
/// assert_eq!(super::is_timeout(&super::Node {
///     clock: Instant::now(),
///     delay: 600,
///     channel: 0,
/// }), false);
/// ```
fn is_timeout(n: &Node) -> bool {
    n.clock.elapsed().as_secs() >= n.delay
}