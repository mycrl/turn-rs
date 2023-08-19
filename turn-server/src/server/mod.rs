pub mod transport;

use crate::config::{Config, Transport};
use crate::monitor::Monitor;
use crate::router::Router;

use std::sync::Arc;

use tokio::net::{TcpListener, UdpSocket};
use turn_rs::Service;

/// start turn server.
///
/// create a specified number of threads,
/// each thread processes udp data separately.
pub async fn run(config: Arc<Config>, monitor: Monitor, service: &Service) -> anyhow::Result<()> {
    let router = Arc::new(Router::default());
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
                ));
            }
            Transport::TCP => {
                tokio::spawn(transport::tcp_processor(
                    TcpListener::bind(i.bind).await?,
                    i.external.clone(),
                    service.clone(),
                    router.clone(),
                    monitor.clone(),
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
