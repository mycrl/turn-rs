mod monitor;
mod tcp;
mod udp;

pub use monitor::*;
use super::config::{
    Config,
    Protocol,
};

use turn_rs::Service;
use std::sync::Arc;
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
    service: &Service,
    config: Arc<Config>,
) -> anyhow::Result<Monitor> {
    let monitor = Monitor::new(config.turn.threads);
    let router = tcp::Router::new();

    for ite in config.turn.interfaces.clone() {
        let service = service.clone();
        match ite.protocol {
            Protocol::UDP => {
                let socket = Arc::new(UdpSocket::bind(ite.bind).await?);
                for i in 0..config.turn.threads {
                    tokio::spawn(udp::processer(
                        service.get_processor(ite.external),
                        monitor.get_sender(i),
                        socket.clone(),
                    ));
                }
            },
            Protocol::TCP => {
                tokio::spawn(tcp::processer(
                    move || service.get_processor(ite.external),
                    TcpListener::bind(ite.bind).await?,
                    router.clone(),
                ));
            },
        }

        log::info!(
            "turn server listening: {}, external: {}, protocol: {:?}",
            ite.bind,
            ite.external,
            ite.protocol,
        );
    }

    log::info!("turn server workers number: {}", config.turn.threads);
    Ok(monitor)
}
