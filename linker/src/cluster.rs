use tokio::net::UdpSocket;
use std::{collections::HashMap, net::SocketAddr};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Cluster {
    socket: Arc<UdpSocket>,
    linkers: Arc<RwLock<HashMap<SocketAddr, ()>>>,
}

impl Cluster {
    
}
