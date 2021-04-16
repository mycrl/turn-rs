use tokio::{
    time::Instant,
    sync::RwLock
};

use std::{
    collections::HashMap,
    net::SocketAddr
};

struct Timer {
    clock: Instant,
    delay: u64
}

pub struct Group {
    code: u32,
    port_timer: RwLock<HashMap<u16, Timer>>,
    channel_timer: RwLock<HashMap<u16, Timer>>>,
    ports: RwLock<HashMap<Arc<SocketAddr, u16>>>,
    channels: RwLock<HashMap<Arc<SocketAddr, u16>>>
}

impl Group {
    pub fn alloc_port(&self, a: &Arc<SocketAddr>) {
        
    }
}