pub mod rpc;

use std::{
    net::{SocketAddr, IpAddr},
    sync::Arc,
};

use anyhow::{
    Result,
    anyhow,
};

use parking_lot::RwLock;
use rpc::{
    Rpc,
    RpcObserver,
    Payload,
    ProxyStateNotifyNode,
    transport::TransportAddr,
};

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProxyOptions {
    pub bind: SocketAddr,
    pub proxy: SocketAddr,
}

pub trait ProxyObserver: Send + Sync {
    fn create_permission(&self, id: u8, from: SocketAddr, peer: SocketAddr);
}

pub struct Proxy {
    nodes: Arc<RwLock<Vec<ProxyStateNotifyNode>>>,
    rpc: Arc<Rpc>,
}

impl Proxy {
    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub async fn new<T>(
        options: &ProxyOptions,
        observer: T,
    ) -> Result<Arc<Self>>
    where
        T: ProxyObserver + 'static,
    {
        let nodes: Arc<RwLock<Vec<ProxyStateNotifyNode>>> = Default::default();
        Ok(Arc::new(Self {
            rpc: Rpc::new(
                TransportAddr {
                    bind: options.bind,
                    proxy: options.proxy,
                },
                RpcObserverExt {
                    observer: Arc::new(observer),
                    nodes: nodes.clone(),
                },
            )
            .await?,
            nodes,
        }))
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub fn in_online_nodes(&self, addr: &IpAddr) -> bool {
        if let Some(node) =
            self.nodes.read().iter().find(|n| &n.external.ip() == addr)
        {
            node.online
        } else {
            false
        }
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub fn send(&self, payload: Payload, to: u8) -> Result<()> {
        self.rpc.send(payload, to)?;
        Ok(())
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub fn create_permission(
        &self,
        from: &SocketAddr,
        peer_addr: &SocketAddr,
    ) -> Result<()> {
        let nodes = self.nodes.read();
        let node = nodes
            .iter()
            .find(|n| &n.external == peer_addr)
            .ok_or_else(|| anyhow!("not found node!"))?;
        self.rpc.send_with_order(
            Payload::CreatePermission {
                id: node.index,
                from: from.clone(),
                peer: peer_addr.clone(),
            },
            node.index,
        )?;

        Ok(())
    }
}

struct RpcObserverExt {
    observer: Arc<dyn ProxyObserver>,
    nodes: Arc<RwLock<Vec<ProxyStateNotifyNode>>>,
}

impl RpcObserver for RpcObserverExt {
    fn on(&self, payload: Payload) {
        match payload {
            Payload::ProxyStateNotify(nodes) => {
                *self.nodes.write() = nodes;
            },
            Payload::CreatePermission {
                id,
                from,
                peer,
            } => {
                self.observer.create_permission(id, from, peer);
            },
        }
    }
}
