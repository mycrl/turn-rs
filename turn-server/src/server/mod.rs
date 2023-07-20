pub mod transport;
pub mod monitor;

pub use self::monitor::*;

use super::router::Router;
use super::config::{
    Transport,
    Config,
};

use std::sync::Arc;

use async_trait::async_trait;
use turn_proxy::rpc::RelayPayloadKind;
use turn_proxy::{
    Proxy,
    ProxyObserver,
    rpc::RelayPayload,
};

use turn_rs::{
    Service,
    StunClass,
};

use tokio::net::{
    TcpListener,
    UdpSocket,
};

#[derive(Clone)]
struct ProxyExt {
    service: Service,
    router: Arc<Router>,
}

#[async_trait]
impl ProxyObserver for ProxyExt {
    async fn relay(&self, payload: RelayPayload) {
        let class = match payload.kind {
            RelayPayloadKind::Message => StunClass::Message,
            RelayPayloadKind::Channel => StunClass::Channel,
        };

        let router = self.service.get_router();
        if let Some(addr) = router.get_port_bound(payload.peer.port()) {
            if let Some(node) = router.get_node(&addr) {
                self.router
                    .send(node.index, class, &addr, &payload.data)
                    .await;
            }
        }
    }
}

/// start turn server.
///
/// create a specified number of threads,
/// each thread processes udp data separately.
///
/// # Example
///
/// ```ignore
/// let config = Config::new()
/// let service = Service::new(/* ... */);;
///
/// // run(&service, config).await?
/// ```
pub async fn run(
    config: Arc<Config>,
    monitor: Monitor,
    service: &Service,
) -> anyhow::Result<()> {
    let router = Arc::new(Router::new());
    let proxy = if let Some(cfg) = &config.proxy {
        Some(
            Proxy::new(
                &cfg,
                ProxyExt {
                    service: service.clone(),
                    router: router.clone(),
                },
            )
            .await?,
        )
    } else {
        None
    };

    for i in config.turn.interfaces.clone() {
        let service = service.clone();
        match i.transport {
            Transport::UDP => {
                tokio::spawn(transport::udp_processor(
                    UdpSocket::bind(i.bind).await?,
                    i.clone(),
                    service.clone(),
                    router.clone(),
                    monitor.clone(),
                    proxy.clone(),
                ));
            },
            Transport::TCP => {
                tokio::spawn(transport::tcp_processor(
                    TcpListener::bind(i.bind).await?,
                    i.clone(),
                    service.clone(),
                    router.clone(),
                    monitor.clone(),
                    proxy.clone(),
                ));
            },
        }

        log::info!(
            "turn server listening: addr={}, external={}, transport={:?}",
            i.bind,
            i.external,
            i.transport,
        );
    }

    Ok(())
}
