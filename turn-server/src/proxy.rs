use std::sync::Arc;

use async_trait::async_trait;
use turn_proxy::ProxyObserver;
use turn_rs::Service;

use crate::router::Router;

#[derive(Clone)]
pub struct ProxyExt {
    pub service: Service,
    pub router: Arc<Router>,
}

#[async_trait]
impl ProxyObserver for ProxyExt {
    async fn relay(&self, payload: &[u8]) {}
}
