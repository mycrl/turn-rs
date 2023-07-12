pub mod rpc;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use rpc::{Rpc, RpcObserver, Payload};

pub struct ProxyNode {
    pub bind: SocketAddr,
    pub external: SocketAddr,
}

pub struct ProxyOptions {
    pub nodes: Vec<ProxyNode>,
    pub bind: SocketAddr,
}

pub struct Proxy {
    options: ProxyOptions,
    rpc: Arc<Rpc>,
}

impl Proxy {
    pub async fn new(options: ProxyOptions) -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            rpc: Rpc::new(options.bind, RpcObserverExt).await?,
            options,
        }))
    }

    pub fn in_proxy_node(&self, addr: &SocketAddr) -> bool {
        self.options.nodes.iter().any(|n| &n.external == addr)
    }

    pub fn create_permission(&self, peer_addr: &SocketAddr) {}
}

struct RpcObserverExt;
impl RpcObserver for RpcObserverExt {
    fn on(&self, payload: Payload) {
        match payload {
            Payload::ProxyStateNotify { nodes } => {

            }
            _ => ()
        }
    }
}
