use std::net::SocketAddr;
use serde::{
    Deserialize,
    Serialize
};

/// RPC service type.
#[repr(u8)]
pub enum Service {
    /// auth request.
    Auth = 0,
    /// get node info.
    Get = 1,
    /// remove node.
    Remove = 2,
}

/// universal request.
#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct Request {
    pub addr: SocketAddr
}

/// auth request.
#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct AuthRequest {
    pub addr: SocketAddr,
    pub username: String
}

/// auth response.
#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct Auth {
    pub password: String,
    pub group: u32,
}

/// session node info.
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
    pub fn from(n: &crate::state::Node) -> Self {
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
