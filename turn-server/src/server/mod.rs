pub mod monitor;
pub mod transport;

pub use self::monitor::*;

use super::config::{Config, Transport};
use super::router::Router;
use crate::proxy::ProxyExt;

use std::sync::Arc;

use tokio::net::{TcpListener, UdpSocket};
use turn_proxy::Proxy;
use turn_rs::Service;

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
pub async fn run(config: Arc<Config>, monitor: Monitor, service: &Service) -> anyhow::Result<()> {
    let router = Arc::new(Router::default());
    let proxy = if let Some(cfg) = &config.proxy {
        Some(Proxy::new(cfg, ProxyExt::new(service.clone(), router.clone())).await?)
    } else {
        None
    };

    for i in config.turn.interfaces.clone() {
        let service = service.clone();
        match i.transport {
            Transport::UDP => {
                tokio::spawn(transport::udp_processor(
                    UdpSocket::bind(i.bind).await?,
                    i.external.clone(),
                    service.clone(),
                    router.clone(),
                    monitor.clone(),
                    proxy.clone(),
                ));
            }
            Transport::TCP => {
                tokio::spawn(transport::tcp_processor(
                    TcpListener::bind(i.bind).await?,
                    i.external.clone(),
                    service.clone(),
                    router.clone(),
                    monitor.clone(),
                    proxy.clone(),
                ));
            }
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
