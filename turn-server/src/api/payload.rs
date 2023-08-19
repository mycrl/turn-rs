use std::net::SocketAddr;

use crate::config::Interface;

use serde::*;
use turn_rs::Node as TNode;

#[rustfmt::skip]
pub static SOFTWARE: &str = concat!(
    env!("CARGO_PKG_NAME"), 
    ":", 
    env!("CARGO_PKG_VERSION")
);

#[derive(Serialize)]
pub struct Stats {
    /// Software information, usually a name and version string.
    pub software: String,
    /// The listening interfaces of the turn server.
    pub interfaces: Vec<Interface>,
    /// The running time of the server, in seconds.
    pub uptime: u64,
    /// Turn server port pool capacity.
    pub port_capacity: u16,
    /// The number of ports that the turn server has classified.
    pub port_allocated: u16,
    /// The partition where the turn server resides.
    pub realm: String,
}

/// node information in the turn server
#[derive(Serialize)]
pub struct Node {
    /// Username for the current INodeion.
    pub username: String,
    /// The user key for the current INodeion.
    pub password: String,
    /// The lifetime of the current user.
    pub lifetime: u64,
    /// The active time of the current user, in seconds.
    pub timer: u64,
    /// List of assigned channel numbers.
    pub allocated_channels: Vec<u16>,
    /// List of assigned port numbers.
    pub allocated_ports: Vec<u16>,
}

impl From<TNode> for Node {
    fn from(value: TNode) -> Self {
        Node {
            timer: value.timer.elapsed().as_secs(),
            username: value.username.clone(),
            allocated_channels: value.channels,
            allocated_ports: value.ports,
            password: value.password,
            lifetime: value.lifetime,
        }
    }
}

#[rustfmt::skip]
#[derive(Serialize)]
pub enum Events<'a> {
    /// allocate request
    Allocated {
        addr: &'a SocketAddr,
        name: &'a str,
        port: u16,
    },
    /// binding request
    Binding { 
        addr: &'a SocketAddr 
    },
    /// channel binding request
    ChannelBind {
        addr: &'a SocketAddr,
        name: &'a str,
        number: u16,
    },
    /// create permission request
    CreatePermission {
        addr: &'a SocketAddr,
        name: &'a str,
        relay: &'a SocketAddr,
    },
    /// refresh request
    Refresh {
        addr: &'a SocketAddr,
        name: &'a str,
        time: u32,
    },
    /// node exit
    Abort { 
        addr: &'a SocketAddr, 
        name: &'a str 
    },
}

impl Events<'_> {
    pub const fn kind_name(&self) -> &'static str {
        match *self {
            Self::Allocated { .. } => "allocated",
            Self::Binding { .. } => "binding",
            Self::ChannelBind { .. } => "channel_bind",
            Self::CreatePermission { .. } => "create_permission",
            Self::Refresh { .. } => "refresh",
            Self::Abort { .. } => "abort",
        }
    }
}
