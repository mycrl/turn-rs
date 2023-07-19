pub mod transport;
pub mod monitor;

pub use self::monitor::*;

use super::router::Router;
use super::config::{
    Transport,
    Config,
};

use std::net::SocketAddr;
use std::sync::Arc;
use turn_proxy::{
    Proxy,
    ProxyObserver,
};

use turn_rs::Service;
use tokio::net::{
    TcpListener,
    UdpSocket,
};

#[derive(Clone)]
struct ProxyExt {
    service: Service,
    router: Arc<Router>,
}

impl ProxyObserver for ProxyExt {
    fn create_permission(&self, id: u8, from: SocketAddr, peer: SocketAddr) {
        self.service
            .get_router()
            .bind_port(&from, peer.port(), Some(id));
    }

    fn relay(&self, buf: &[u8]) {}
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
