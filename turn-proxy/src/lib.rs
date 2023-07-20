pub mod rpc;

use std::sync::Arc;
use std::net::{
    SocketAddr,
    IpAddr,
};

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::RwLock;
use rpc::{
    RelayPayload,
    RelayPayloadKind,
};
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

#[async_trait]
pub trait ProxyObserver: Send + Sync {
    async fn relay(&self, payload: RelayPayload);
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
        kind: RelayPayloadKind,
        data: &[u8],
    ) -> Result<()> {
        self.rpc
            .send(
                RelayPayload {
                    data: data.to_vec(),
                    kind,
                    from,
                    peer,
                },
                node.index,
            )
            .await?;
        Ok(())
    }
}

struct RpcObserverExt {
    observer: Arc<dyn ProxyObserver>,
    nodes: Arc<RwLock<Vec<Arc<ProxyStateNotifyNode>>>>,
}

#[async_trait]
impl RpcObserver for RpcObserverExt {
    fn on(&self, req: Request) {
        match req {
            Request::ProxyStateNotify(nodes) => {
                log::info!("received state sync from proxy: state={:?}", nodes);
                *self.nodes.write() = nodes.into_iter().map(Arc::new).collect()
            },
        }
    }

    async fn on_relay(&self, payload: RelayPayload) {
        self.observer.relay(payload).await;
    }
}
