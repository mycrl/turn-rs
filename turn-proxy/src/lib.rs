pub mod rpc;

use std::{
    net::SocketAddr,
    sync::Arc,
};

use anyhow::{
    Result,
    anyhow,
};

use turn_rs::Router;
use parking_lot::RwLock;
use rpc::{
    Rpc,
    RpcObserver,
    Payload,
    ProxyStateNotifyNode,
};

pub struct ProxyOptions {
    pub proxy: SocketAddr,
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
    pub async fn new(
        options: &ProxyOptions,
        router: Arc<Router>,
    ) -> Result<Arc<Self>> {
        let nodes: Arc<RwLock<Vec<ProxyStateNotifyNode>>> = Default::default();
        Ok(Arc::new(Self {
            rpc: Rpc::new(
                options.proxy,
                RpcObserverExt {
                    nodes: nodes.clone(),
                    router,
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
    pub fn in_nodes(&self, addr: &SocketAddr) -> bool {
        self.nodes.read().iter().any(|n| &n.external == addr)
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
    pub fn get_node_online(&self, addr: &SocketAddr) -> bool {
        if let Some(node) =
            self.nodes.read().iter().find(|n| &n.external == addr)
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
        self.rpc.send(
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
    router: Arc<Router>,
    nodes: Arc<RwLock<Vec<ProxyStateNotifyNode>>>,
}

impl RpcObserver for RpcObserverExt {
    fn on(&self, payload: Payload) {
        match payload {
            Payload::ProxyStateNotify {
                nodes,
            } => {
                *self.nodes.write() = nodes;
            },
            Payload::CreatePermission {
                id,
                from,
                peer,
            } => {
                if self
                    .router
                    .bind_port_from_proxy(&from, peer.port(), id)
                    .is_none()
                {}
            },
            _ => (),
        }
    }
}
