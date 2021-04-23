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
    pub ports: Vec<u16>,
    pub channels: Vec<u16>,
}

pub struct Bucket {
    size: usize,
    port: RandomPort,
    ports: HashMap<Addr, >
}

impl Bucket {
    
}