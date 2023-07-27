use crate::router::Router;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use faster_stun::Decoder;
use turn_proxy::ProxyObserver;
use turn_rs::Service;

pub struct ProxyExt {
    service: Service,
    router: Arc<Router>,
    decoder: Mutex<Decoder>,
}

impl ProxyExt {
    pub fn new(service: Service, router: Arc<Router>) -> Self {
        Self {
            decoder: Mutex::new(Decoder::new()),
            service,
            router,
        }
    }
}

#[async_trait]
impl ProxyObserver for ProxyExt {
    async fn relay(&self, payload: &[u8]) {}
}
