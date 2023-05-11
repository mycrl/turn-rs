mod monitor;

pub use monitor::*;

use tokio::net::UdpSocket;
use super::config::Config;
use std::{
    io::ErrorKind::*,
    sync::Arc,
};

use turn_rs::{
    Service,
    Processor,
};

/// udp socket process thread.
///
/// read the data packet from the UDP socket and hand
/// it to the proto for processing, and send the processed
/// data packet to the specified address.
async fn fork_socket(
    mut processor: Processor,
    sender: MonitorSender,
    socket: Arc<UdpSocket>,
) -> anyhow::Result<()> {
    let mut buf = vec![0u8; 4096];

    loop {
        // TODO: An error will also be reported when the remote host is shut
        // down, which is not processed yet, but a warning will be
        // issued.
        let (size, addr) = match socket.recv_from(&mut buf).await {
            Ok(s) => s,
            Err(e) => {
                if e.kind() != ConnectionReset {
                    return Err(e.into());
                } else {
                    continue;
                }
            },
        };

        sender.send(Payload::Receive);
        log::trace!("udp socket receive: size={}, addr={:?}", size, addr);

        // The stun message requires at least 4 bytes. (currently the smallest
        // stun message is channel data, excluding content)
        if size >= 4 {
            if let Ok(Some((res, addr))) =
                processor.process(&buf[..size], addr).await
            {
                if let Err(e) = socket.send_to(res, addr.as_ref()).await {
                    if e.kind() != ConnectionReset {
                        return Err(e.into());
                    }
                } else {
                    sender.send(Payload::Send);
                    log::trace!(
                        "udp socket relay: size={}, addr={:?}",
                        res.len(),
                        addr.as_ref()
                    );
                }

                continue;
            }
        }

        sender.send(Payload::Failed);
        log::trace!(
            "udp socket process failed: size={}, addr={:?}",
            size,
            addr
        );
    }
}

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
    for ite in &config.turn.interfaces {
        let socket = Arc::new(UdpSocket::bind(ite.bind).await?);
        for index in 0..config.turn.threads {
            tokio::spawn(fork_socket(
                service.get_processor(ite.external),
                monitor.get_sender(index),
                socket.clone(),
            ));
        }

        log::info!(
            "turn server listening: {}, external: {}",
            ite.bind,
            ite.external
        );
    }

    log::info!("turn server workers number: {}", config.turn.threads);
    Ok(monitor)
}
