mod service;

pub use service::*;
use tokio::net::TcpStream;
use transport::Rpc as Trpc;
use anyhow::{
    Result,
    anyhow
};

use super::{
    state::State,
    config::Conf,
};

use std::{
    net::SocketAddr,
    sync::Arc,
};

/// RPC communication with the control center server.
pub struct Rpc {
    inner: Arc<Trpc>,
    state: Arc<State>,
}

impl Rpc {
    #[rustfmt::skip]
    pub async fn new(c: &Arc<Conf>, s: &Arc<State>) -> Result<Arc<Self>> {
        let (reader, writer) = TcpStream::connect(c.controls)
            .await?
            .into_split();
        let myself = Arc::new(Self {
            inner: Trpc::new(reader, writer).run(),
            state: s.clone(),
        });

        myself.clone().run_get_service().await;
        myself.clone().run_remove_service().await;
        Ok(myself)
    }

    /// get auth info.
    ///
    /// send username and socketaddr to remote,
    /// control center service callback password and group id.
    #[rustfmt::skip]
    pub async fn auth(&self, u: &str, a: &SocketAddr) -> Result<Auth> {
        self.inner.call(Service::Auth as u8, &AuthRequest {
            username: u.to_string(),
            addr: *a
        }).await
    }

    /// start get node info service.
    #[rustfmt::skip]
    async fn run_get_service(self: Arc<Self>) {
        self.inner.clone().bind(Service::Get as u8, move |req: Request| {
            let state = self.state.clone();
            async move {
                state.base_table.read().await
                    .get(&req.addr)
                    .map(Node::from)
                    .ok_or_else(|| anyhow!("not found"))
            }
        }).await
    }

    /// start remove node service.
    #[rustfmt::skip]
    async fn run_remove_service(self: Arc<Self>) {
        self.inner.clone().bind(Service::Remove as u8, move |req: Request| {
            let state = self.state.clone();
            async move {
                state.remove(&Arc::new(req.addr)).await;
                Ok(())
            }
        }).await
    }
}
