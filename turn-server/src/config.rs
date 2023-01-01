use std::net::SocketAddr;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(
    name = "TURN (Traversal Using Relays around NAT)",
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS")
)]
pub struct Config {
    /// realm:
    ///
    /// specify the domain where the server is located.
    /// for a single node, this configuration is fixed,
    /// but each node can be configured as a different domain.
    /// this is a good idea to divide the nodes by namespace.
    #[clap(long)]
    #[clap(env = "TURN_REALM")]
    #[clap(default_value = "localhost")]
    pub realm: String,

    /// external:
    ///
    /// specify the node external address and port.
    /// for the case of exposing the service to the outside,
    /// you need to manually specify the server external IP
    /// address and service listening port.
    #[clap(long)]
    #[clap(env = "TURN_EXTERNAL")]
    #[clap(default_value = "127.0.0.1:3478")]
    pub external: SocketAddr,

    /// bind:
    ///
    /// the address and port bound by UDP Server.
    /// currently, it does not support binding multiple
    /// addresses at the same time. the bound address
    /// supports ipv4 and ipv6.
    #[clap(long)]
    #[clap(env = "TURN_BIND")]
    #[clap(default_value = "127.0.0.1:3478")]
    pub bind: SocketAddr,

    /// controller bind:
    ///
    /// This option specifies the http server binding address used to control
    /// the turn server.
    ///
    /// Warn: This http server does not contain
    /// any means of authentication, and sensitive information and dangerous
    /// operations can be obtained through this service, please do not expose
    /// it directly to an unsafe environment.
    #[clap(long)]
    #[clap(env = "TURN_CONTROLLER_BIND")]
    #[clap(default_value = "127.0.0.1:3000")]
    pub controller_bind: SocketAddr,

    /// external controller bind:
    ///
    /// This option is used to specify the http address of the external control
    /// service.
    ///
    /// Warn: This http server does not contain
    /// any means of authentication, and sensitive information and dangerous
    /// operations can be obtained through this service, please do not expose
    /// it directly to an unsafe environment.
    #[clap(long)]
    #[clap(env = "TURN_EXT_CONTROLLER_BIND")]
    #[clap(default_value = "http://127.0.0.1:3000")]
    pub ext_controller_bind: String,

    /// static certificate file path:
    ///
    /// The internal format of the file is TOML, and the content is
    /// `[username]="[password]"`. This option can be used to specify the
    /// static identity authentication information used by the turn server for
    /// verification. Note: this is a high-priority authentication method, turn
    /// The server will try to use static authentication first, and then use
    /// external control service authentication.
    #[clap(long)]
    #[clap(env = "TURN_CERT_FILE")]
    pub cert_file: Option<String>,

    /// threads:
    ///
    /// by default, the thread pool is used to process UDP packets.
    /// because UDP uses SysCall to ensure concurrency security,
    /// using multiple threads may not bring a very significant
    /// performance improvement, but setting the number of CPU
    /// cores can process data to the greatest extent package.
    #[clap(long)]
    #[clap(env = "TURN_THREADS")]
    pub threads: Option<usize>,
}
