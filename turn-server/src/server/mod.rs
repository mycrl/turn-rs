mod transport;
mod router;

pub use self::router::Router;

use crate::monitor::Monitor;
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
/// ```no_run
/// let config = Config::new()
/// let service = Service::new(/* ... */);;
///
/// // run(&service, config).await?
/// ```
pub async fn run(
    _monitor: &Monitor,
    service: &Service,
    config: Arc<Config>,
) -> anyhow::Result<()> {
    let router = Arc::new(Router::new());
    for interface in config.turn.interfaces.clone() {
        let service = service.clone();
        match interface.transport {
            Transport::UDP => {
                tokio::spawn(transport::udp_processor(
                    UdpSocket::bind(interface.bind).await?,
                    interface.clone(),
                    service.clone(),
                    router.clone(),
                ));
            },
            Transport::TCP => {
                tokio::spawn(transport::tcp_processor(
                    TcpListener::bind(interface.bind).await?,
                    move |index| {
                        service.get_processor(index, interface.external)
                    },
                    router.clone(),
                ));
            },
        }

        log::info!(
            "turn server listening: addr={}, external={}, transport={:?}",
            interface.bind,
            interface.external,
            interface.transport,
        );
    }

    log::info!("turn server workers: number={}", config.turn.threads);
    Ok(())
}
