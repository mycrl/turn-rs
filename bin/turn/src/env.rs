use clap::Parser;
use std::{
    net::SocketAddr,
    sync::Arc
};

#[derive(Parser)]
#[clap(
    name = "TURN (Traversal Using Relays around NAT)",
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS")
)]
pub struct Environment {
    /// realm:
    ///
    /// specify the domain where the server is located. 
    /// for a single node, this configuration is fixed, 
    /// but each node can be configured as a different domain. 
    /// this is a good idea to divide the nodes by namespace.
    #[clap(
        about = "service realm name",
        default_value = option_env!("TURN_REALM")
            .unwrap_or("localhost")
    )]
    pub realm: String,
    /// external:
    ///
    /// specify the node external address and port. 
    /// for the case of exposing the service to the outside, 
    /// you need to manually specify the server external IP 
    /// address and service listening port.
    #[clap(
        about = "service external address and port",
        default_value = option_env!("TURN_EXTERNAL")
            .unwrap_or("127.0.0.1:3478")
    )]
    pub external: SocketAddr,
    /// listen:
    ///
    /// the address and port bound by UDP Server. 
    /// currently, it does not support binding multiple 
    /// addresses at the same time. the bound address 
    /// supports ipv4 and ipv6.
    #[clap(
        about = "service bind address and port",
        default_value = option_env!("TURN_LISTEN")
            .unwrap_or("127.0.0.1:3478")
    )]
    pub listen: SocketAddr,
    /// nats:
    ///
    /// specify the remote control service. 
    /// the control service is very important. 
    /// if it is separated from it, 
    /// the service will only have the basic STUN binding function. 
    /// functions such as authorization authentication and port 
    /// allocation require communication with the control center.
    #[clap(
        about = "nats server connection url",
        default_value = option_env!("TURN_NATS")
            .unwrap_or("127.0.0.1:4222")
    )]
    pub nats: String,
    /// threads:
    ///
    /// by default, the thread pool is used to process UDP packets. 
    /// because UDP uses SysCall to ensure concurrency security, 
    /// using multiple threads may not bring a very significant 
    /// performance improvement, but setting the number of CPU 
    /// cores can process data to the greatest extent package.
    #[clap(about = "runtime threads size")]
    pub threads: Option<usize>,
}

impl Environment {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::parse())
    }
}
