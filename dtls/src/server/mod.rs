mod session;

use anyhow::Result;
use session::Session;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::Arc;

pub struct Server {
    inner: Arc<UdpSocket>,
    sessions: HashMap<SocketAddr, Session<'static>>
}

impl Server {
    pub async fn new(addr: SocketAddr) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(UdpSocket::bind(addr)?),
            sessions: HashMap::with_capacity(1024),
        })
    }
}
