use clap::Clap;
use std::{
    net::SocketAddr,
    sync::Arc
};

/// cli args.
#[derive(Clap)]
#[clap(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS")
)]
pub struct Argv {
    /// specify the domain where the server is located. 
    /// for a single node, this configuration is fixed, 
    /// but each node can be configured as a different domain. 
    /// this is a good idea to divide the nodes by namespace.
    #[clap(long)]
    #[clap(default_value = "localhost")]
    #[clap(about = "service realm name")]
    pub realm: String,
    /// specify the node external address and port. 
    /// for the case of exposing the service to the outside, 
    /// you need to manually specify the server external IP 
    /// address and service listening port.
    #[clap(long)]
    #[clap(default_value = "127.0.0.1:3478")]
    #[clap(about = "service external address and port")]
    pub external: SocketAddr,
    /// the address and port bound by UDP Server. 
    /// currently, it does not support binding multiple 
    /// addresses at the same time. the bound address 
    /// supports ipv4 and ipv6.
    #[clap(long)]
    #[clap(default_value = "127.0.0.1:3478")]
    #[clap(about = "service bind address and port")]
    pub listen: SocketAddr,
    /// specify the remote control service. 
    /// the control service is very important. 
    /// if it is separated from it, 
    /// the service will only have the basic STUN binding function. 
    /// functions such as authorization authentication and port 
    /// allocation require communication with the control center.
    #[clap(long)]
    #[clap(default_value = "127.0.0.1:4222")]
    #[clap(about = "nats server connection url")]
    pub nats: String,
    /// tshe buffer size is used to determine the maximum 
    /// data allocation size (byte) owned by each thread pool. 
    /// in actual use, it is recommended to configure this 
    /// value to 4096. a larger space will be easier to deal 
    /// with more complex MTU situations, although most of 
    /// the time The space utilization rate is not high.
    #[clap(long)]
    #[clap(default_value = "1280")]
    #[clap(about = "udp cache buffer size")]
    pub buffer: usize,
    /// by default, the thread pool is used to process UDP packets. 
    /// because UDP uses SysCall to ensure concurrency security, 
    /// using multiple threads may not bring a very significant 
    /// performance improvement, but setting the number of CPU 
    /// cores can process data to the greatest extent package.
    #[clap(long)]
    #[clap(about = "runtime threads size")]
    pub threads: Option<usize>,
}

impl Argv {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::parse())
    }
}
