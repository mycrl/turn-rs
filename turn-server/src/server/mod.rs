pub mod transport;
pub mod monitor;

pub use self::monitor::*;

use super::router::Router;
use super::config::{
    Transport,
    Config,
};

use std::sync::Arc;
use turn_rs::Service;
use tokio::net::{
    TcpListener,
    UdpSocket,
};

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
    monitor: Monitor,
    service: &Service,
    config: Arc<Config>,
) -> anyhow::Result<()> {
    let router = Arc::new(Router::new());
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
                ));
            },
            Transport::TCP => {
                tokio::spawn(transport::tcp_processor(
                    TcpListener::bind(i.bind).await?,
                    i.clone(),
                    service.clone(),
                    router.clone(),
                    monitor.clone(),
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
