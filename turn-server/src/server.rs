use tokio::net::UdpSocket;
use super::args::Args;
use std::sync::Arc;
use turn::{
    Service,
    Processor,
};

/// thread poll.
///
/// read the data packet from the UDP socket and hand
/// it to the proto for processing, and send the processed
/// data packet to the specified address.
///
/// # Example
///
/// ```no_run
/// let c = env::Environment::generate()?;
/// let t = broker::Broker::new(&c).await?;
/// let s = state::State::new(t);
///
/// let thread_local = SocketLocal {
///     state: s,
///     conf: c,
/// };
///
/// let s = Arc::new(UdpSocket::bind(c.listen).await?);
/// tokio::spawn(start(thread_local, &s));
/// ```
pub async fn start(
    mut processor: Processor,
    socket: Arc<UdpSocket>,
) -> anyhow::Result<()> {
    let mut reader = vec![0u8; 4096];

    loop {
        if let Ok((size, addr)) = socket.recv_from(&mut reader).await {
            if size >= 4 {
                if let Ok(Some((buf, target))) =
                    processor.process(&reader[..size], addr).await
                {
                    socket.send_to(buf, target.as_ref()).await?;
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
/// let c = env::Environment::generate()?;
/// let t = broker::Broker::new(&c).await?;
/// let s = state::State::new(t);
///
/// // run(c, s).await?
/// ```
pub async fn run(args: &Arc<Args>, service: &Service) -> anyhow::Result<()> {
    let socket = Arc::new(UdpSocket::bind(args.bind).await?);
    let threads = get_threads(args.threads);

    for _ in 0..threads {
        tokio::spawn(start(service.get_processor(), socket.clone()));
    }

    log::info!("threads size {} is runing", threads);
    log::info!("udp bind to {}", args.bind);

    Ok(())
}
