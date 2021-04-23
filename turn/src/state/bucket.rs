use random_port::RandomPort;
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
}

pub struct Bucket {
    size: usize,
    port: RandomPort,
    nodes: HashMap<Addr, Node>
}

impl Bucket {
    
}