mod tcp;
mod udp;
mod tls;

use std::sync::Arc;
use faster_stun::attribute::Transport;

use self::tcp::Router;
use crate::monitor::Monitor;
use super::config::{
    Config,
    self,
};

use turn_rs::Service;
use tokio::net::{
    TcpListener,
    UdpSocket,
};

/// start udp server.
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
            config::Transport::UDP => {
                let socket = Arc::new(UdpSocket::bind(ite.bind).await?);
                for _ in 0..config.turn.threads {
                    tokio::spawn(udp::processer(
                        service.get_processor(ite.external, Transport::UDP),
                        monitor.get_sender().await,
                        socket.clone(),
                    ));
                }
            },
            config::Transport::TCP => {
                tokio::spawn(tcp::processer(
                    move || service.get_processor(ite.external, Transport::TCP),
                    monitor.get_sender().await,
                    router.clone(),
                    TcpListener::bind(ite.bind).await?,
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
