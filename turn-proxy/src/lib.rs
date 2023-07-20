pub mod rpc;

use std::sync::Arc;
use std::net::{
    SocketAddr,
    IpAddr,
};

use anyhow::Result;
use parking_lot::RwLock;
use rpc::RelayPayload;
use rpc::{
    Rpc,
    Request,
    RpcObserver,
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
    fn relay<'a>(&'a self, payload: RelayPayload<'a>);
}

#[derive(Clone)]
pub struct Proxy {
    nodes: Arc<RwLock<Vec<Arc<ProxyStateNotifyNode>>>>,
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
    pub async fn new<T>(options: &ProxyOptions, observer: T) -> Result<Self>
    where
        T: ProxyObserver + 'static,
    {
        let nodes: Arc<RwLock<Vec<Arc<ProxyStateNotifyNode>>>> =
            Default::default();
        log::info!(
            "create proxy mod: bind={}, proxy={}",
            options.bind,
            options.proxy
        );

        Ok(Self {
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
        })
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
    pub fn get_online_node(
        &self,
        addr: &IpAddr,
    ) -> Option<Arc<ProxyStateNotifyNode>> {
        self.nodes
            .read()
            .iter()
            .find(|n| &n.external.ip() == addr)
            .cloned()
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
    pub async fn relay(
        &self,
        node: &ProxyStateNotifyNode,
        from: SocketAddr,
        peer: SocketAddr,
        data: &[u8],
    ) -> Result<()> {
        self.rpc
            .send(
                RelayPayload {
                    from,
                    peer,
                    data,
                },
                node.index,
            )
            .await?;
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
        node: &ProxyStateNotifyNode,
        from: &SocketAddr,
        peer: &SocketAddr,
    ) -> Result<()> {
        self.rpc.send_with_order(
            Request::CreatePermission {
                id: node.index,
                from: from.clone(),
                peer: peer.clone(),
            },
            node.index,
        )?;

        Ok(())
    }
}

struct RpcObserverExt {
    observer: Arc<dyn ProxyObserver>,
    nodes: Arc<RwLock<Vec<Arc<ProxyStateNotifyNode>>>>,
}

impl RpcObserver for RpcObserverExt {
    fn on(&self, req: Request) {
        match req {
            Request::ProxyStateNotify(nodes) => {
                log::info!("received state sync from proxy: state={:?}", nodes);
                *self.nodes.write() = nodes.into_iter().map(Arc::new).collect()
            },
            Request::CreatePermission {
                id,
                from,
                peer,
            } => {
                self.observer.create_permission(id, from, peer);
                log::info!(
                    "received create permission from proxy: id={}, from={}, \
                     peer={}",
                    id,
                    from,
                    peer
                );
            },
        }
    }

    fn on_relay<'a>(&'a self, payload: RelayPayload<'a>) {
        self.observer.relay(payload);
    }
}
