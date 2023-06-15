mod transport;
mod router;

use std::sync::Arc;
use self::router::Router;
use crate::monitor::Monitor;
use super::config::{
    Transport,
    Config,
};

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
    monitor: &Monitor,
    service: &Service,
    config: Arc<Config>,
) -> anyhow::Result<()> {
    let router = Router::new().await?;

    for ite in config.turn.interfaces.clone() {
        let service = service.clone();
        match ite.transport {
            Transport::UDP => {
                // Under the udp protocol, the processing function is forked
                // according to the number of threads, and multiple threads read
                // and write the same udp socket at the same time.
                let socket = Arc::new(UdpSocket::bind(ite.bind).await?);
                for _ in 0..config.turn.threads {
                    tokio::spawn(transport::udp_processor(
                        service.get_processor(ite.external),
                        monitor.get_sender().await,
                        router.clone(),
                        socket.clone(),
                    ));
                }
            },
            Transport::TCP => {
                // Unlike udp, tcp cannot be processed with a fixed number of
                // tasks, so the listener is still directly handed over to the
                // processing function.
                let listener = TcpListener::bind(ite.bind).await?;
                tokio::spawn(transport::tcp_processor(
                    move || service.get_processor(ite.external),
                    monitor.get_sender().await,
                    router.clone(),
                    listener,
                ));
            },
        }

        log::info!(
            "turn server listening: addr={}, external={}, transport={:?}",
            ite.bind,
            ite.external,
            ite.transport,
        );
    }

    log::info!("turn server workers: number={}", config.turn.threads);
    Ok(())
}
