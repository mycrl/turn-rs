mod monitor;

pub use monitor::*;

use tokio::net::UdpSocket;
use super::config::Config;
use std::sync::Arc;
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
    sender: MonitorSender,
    mut processor: Processor,
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
                log::warn!("udp socket receive error: {:?}", e);
                continue;
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
                // TODO: An error will also be reported when the remote host is
                // shut down, which is not processed yet, but a
                // warning will be issued.
                if let Err(e) = socket.send_to(res, addr.as_ref()).await {
                    log::warn!("udp socket send error: {:?}", e);
                    sender.send(Payload::Failed);
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

/// get thread num.
///
/// by default, the thread pool is used to process UDP packets.
/// because UDP uses SysCall to ensure concurrency security,
/// using multiple threads may not bring a very significant
/// performance improvement, but setting the number of CPU
/// cores can process data to the greatest extent package.
fn get_threads(threads: Option<usize>) -> usize {
    threads.unwrap_or_else(num_cpus::get)
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
/// // run(&service, config ).await?
/// ```
pub async fn run(
    service: &Service,
    config: Arc<Config>,
) -> anyhow::Result<Monitor> {
    let socket = Arc::new(UdpSocket::bind(config.bind).await?);
    let threads = get_threads(config.threads);
    let monitor = Monitor::new(threads);

    for index in 0..threads {
        tokio::spawn(fork_socket(
            monitor.get_sender(index),
            service.get_processor(),
            socket.clone(),
        ));
    }

    log::info!("turn server workers number: {}", threads);
    log::info!("turn server listening: {}", config.bind);
    Ok(monitor)
}
