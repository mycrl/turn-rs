use crate::state::Node as Base;
use std::net::SocketAddr;
use serde::{
    Deserialize,
    Serialize
};

/// 触发器
#[tarpc::service]
pub trait Trigger {
    async fn auth(req: AuthRequest) -> Auth;
}

/// 状态服务
#[tarpc::service]
pub trait State {
    async fn get(addr: SocketAddr) -> Option<Node>;
    async fn remove(addr: SocketAddr);
}

/// 认证请求
///
/// * `addr` 客户端地址
/// * `username` 用户名
#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct AuthRequest {
    pub addr: SocketAddr,
    pub username: String
}

/// 认证信息
///
/// * `password` 密钥
/// * `group` 分组ID
#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct Auth {
    pub password: String,
    pub group: u32,
}

/// 节点
///
/// * `group` 分组ID
/// * `delay` 超时时间
/// * `clock` 内部时钟
/// * `password` 密钥
/// * `ports` 分配端口列表
/// * `channels` 分配频道列表
#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct Node {
    pub group: u32,
    pub delay: u64,
    pub clock: u64,
    pub ports: Vec<u16>,
    pub channels: Vec<u16>,
    pub password: String,
}

impl Node {
    /// 获取认证信息
    ///
    /// # Example
    ///
    /// ```test(node_from)
    /// use crate::state::Node as Base;
    /// use tokio::time::Instant;
    /// use std::sync::Arc;
    /// use super::Node;
    ///
    /// let node = Node::from(&Base {
    ///     group: 0,
    ///     delay: 600,
    ///     ports: vec![],
    ///     channels: vec![],
    ///     password: Arc::new("".to_string()),
    ///     clock: Instant::now(),
    /// });
    /// 
    /// assert_eq!(node.group, 0);
    /// assert_eq!(node.delay, 600);
    /// assert_eq!(node.ports.len(), 0);
    /// assert_eq!(node.channels.len(), 0);
    /// assert!(node.clock <= 1);
    /// ```
    pub fn from(n: &Base) -> Self {
        Self {
            clock: n.clock.elapsed().as_secs(),
            password: n.password.to_string(),
            channels: n.channels.clone(),
            ports: n.ports.clone(),
            delay: n.delay,
            group: n.group,
        }
    }
}
