use crate::state::Node as Base;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// RPC服务定义
#[repr(u8)]
pub enum Service {
    /// 认证请求
    Auth = 0,
    /// 获取节点信息
    Get = 1,
    /// 删除节点
    Remove = 2,
}

/// 请求
#[derive(Debug, Deserialize, Serialize)]
pub struct Request {
    pub addr: SocketAddr,
}

/// 认证请求
#[derive(Debug, Deserialize, Serialize)]
pub struct AuthRequest {
    pub addr: SocketAddr,
    pub username: String,
}

/// 认证信息
#[derive(Debug, Deserialize, Serialize)]
pub struct Auth {
    pub password: String,
    pub group: u32,
}

/// 节点
#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    pub group: u32,
    pub delay: u64,
    pub clock: u64,
    pub ports: Vec<u16>,
    pub channels: Vec<u16>,
    pub password: String,
}

impl Node {
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
