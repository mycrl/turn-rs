mod socket;

use tokio::net::UdpSocket;
use anyhow::Result;
use std::sync::Arc;
use super::{
    args::Args,
    router::Router
};

pub use socket::{
    Socket,
    SocketLocal
};

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
#[rustfmt::skip]
pub async fn run(args: &Arc<Args>, routes: &Arc<Router>) -> Result<()> {
    let s = Arc::new(UdpSocket::bind(args.bind).await?);
    let threads = get_threads(args.threads);
    let tl = SocketLocal {
        router: routes.clone(),
        args: args.clone(),
    };
    
    for _ in 0..threads {
        let mut cx = Socket::builder(tl.clone(), &s);
        tokio::spawn(async move {
            loop {
                cx.poll().await;
            }
        });
    }
    
    log::info!(
        "threads size {} is runing", 
        threads
    );
    
    log::info!(
        "udp bind to {}",
        args.bind
    );

    Ok(())
}
