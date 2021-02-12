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
#[derive(Hash, Eq)]
struct UniquePort(
    /// 分组ID
    u32, 
    /// 端口号
    u16
);

/// 频道标识
#[derive(Hash, Eq)]
struct UniqueChannel(
    /// 客户端地址
    Addr, 
    /// 频道号
    u16
);

/// 节点
pub struct Node {
    /// 分组ID
    pub group: u32,
    
    /// 会话的超时时间
    /// TODO: 目前存在问题，未将超时时间细化到每个端口
    pub delay: u64,
    
    /// 内部时钟，记录刷新的TTL
    pub clock: Instant,
    
    /// 当前会话分配的端口列表
    pub ports: Vec<u16>,
    
    /// 当前会话分配的频道列表
    pub channels: Vec<u16>,
    
    /// 当前会话的密钥
    pub password: Arc<String>,
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
    /// 每个用户分配一个具备超时时间的随机ID
    nonce_table: RwLock<HashMap<Addr, (Arc<String>, Instant)>>,
    
    /// 记录会话与对等端的端口绑定关系
    peer_table: RwLock<HashMap<Addr, HashMap<Addr, u16>>>,
    
    /// 记录分组内频道与地址的绑定关系
    channel_table: RwLock<HashMap<UniqueChannel, Addr>>,
    
    /// 记录分组内端口与地址的绑定关系
    port_table: RwLock<HashMap<UniquePort, Addr>>,
    
    /// 记录分组的端口引用计数以及偏移量
    group_port_rc: RwLock<HashMap<u32, (u16, u16)>>,
    
    /// 基础信息表
    pub base_table: RwLock<HashMap<Addr, Node>>,
}

impl State {
    #[rustfmt::skip]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            channel_table: RwLock::new(HashMap::with_capacity(1024)),
            nonce_table: RwLock::new(HashMap::with_capacity(1024)),
            port_table: RwLock::new(HashMap::with_capacity(1024)),
            group_port_rc: RwLock::new(HashMap::with_capacity(1024)),
            peer_table: RwLock::new(HashMap::with_capacity(1024)),
            base_table: RwLock::new(HashMap::with_capacity(1024)),
        })
    }

    /// 获取随机ID
    #[rustfmt::skip]
    pub async fn get_nonce(&self, a: &Addr) -> Arc<String> {
        if let Some((n, c)) = self.nonce_table.read().await.get(a) {
            // 检查是否已经过期
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

    /// 获取密钥
    pub async fn get_password(&self, a: &Addr) -> Option<Arc<String>> {
        self.base_table.read().await.get(a).map(|n| {
            n.password.clone()
        })
    }

    /// 写入节点信息
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

    /// 绑定对端端口
    ///
    /// 通过自身地址和对端端口建立绑定关系
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

    /// 分配端口
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

        if node.ports.len() == 16383 {
            return None
        }

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
    
    /// 分配频道
    #[rustfmt::skip]
    pub async fn insert_channel(&self, a: Addr, p: u16, channel: u16) -> bool {
        assert!(channel >= 0x4000 && channel <= 0x4FFF);
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
        self.channel_table.write().await.insert(
            UniqueChannel(a, channel),
            addr
        );

        true
    }

    /// 获取对等节点绑定的本地端口
    pub async fn reflect_from_peer(&self, a: &Addr, p: &Addr) -> Option<u16> {
        match self.peer_table.read().await.get(a) {
            Some(peer) => peer.get(p).copied(),
            None => None,
        }
    }
    
    /// 根据自身地址和对端端口获取对等地址
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
    
    /// 刷新节点生命周期
    #[rustfmt::skip]
    pub async fn refresh(&self, a: &Addr, delay: u32) -> bool {
        if delay == 0 {
            self.remove(a).await;
            return true;
        }
        
        self.base_table.write().await.get_mut(a).map(|n| {
            n.clock = Instant::now();
            n.delay = delay as u64;
        }).is_some()
    }
    
    /// 获取对等频道
    #[rustfmt::skip]
    pub async fn reflect_from_channel(&self, a: &Addr, channel: u16) -> Option<Addr> {
        assert!(channel >= 0x4000 && channel <= 0x4FFF);
        self.channel_table.read().await.get(
            &UniqueChannel(a.clone(), channel)
        ).cloned()
    }
    
    /// 删除节点
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
    
    /// 启动实例
    ///
    /// 定时扫描内部列表
    /// 扫描间隔为60秒
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

/// 生成随机ID
fn rand_string() -> String {
    let mut rng = thread_rng();
    let r = std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect::<String>();
    r.to_lowercase()
}

/// 是否超时
fn is_timeout(n: &Node) -> bool {
    n.clock.elapsed().as_secs() >= n.delay
}
