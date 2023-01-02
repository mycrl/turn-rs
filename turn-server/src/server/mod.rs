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
        if let Ok((size, addr)) = socket.recv_from(&mut buf).await {
            sender.send(Payload::Receive);
            if size >= 4 {
                if let Ok(Some((res, addr))) =
                    processor.process(&buf[..size], addr).await
                {
                    socket.send_to(res, addr.as_ref()).await?;
                    sender.send(Payload::Send);
                } else {
                    sender.send(Payload::Failed);
                }
            }
        }
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

    log::info!("udp server listening: {}", config.bind);
    log::info!("udp server workers number: {}", threads);
    Ok(monitor)
}
