use std::{collections::HashMap, fs::read_to_string, net::SocketAddr, str::FromStr};

use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::service::session::ports::PortRange;

/// SSL configuration
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Ssl {
    ///
    /// SSL private key file
    ///
    pub private_key: String,
    ///
    /// SSL certificate chain file
    ///
    pub certificate_chain: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "transport", rename_all = "kebab-case")]
pub enum Interface {
    Tcp {
        listen: SocketAddr,
        ///
        /// external address
        ///
        /// specify the node external address and port.
        /// for the case of exposing the service to the outside,
        /// you need to manually specify the server external IP
        /// address and service listening port.
        ///
        external: SocketAddr,
        ///
        /// Idle timeout
        ///
        /// If no packet is received within the specified number of seconds, the
        /// connection will be closed to prevent resources from being occupied
        /// for a long time.
        #[serde(default = "Interface::idle_timeout")]
        idle_timeout: u32,
        ///
        /// SSL configuration
        ///
        #[serde(default)]
        ssl: Option<Ssl>,
    },
    Udp {
        listen: SocketAddr,
        ///
        /// external address
        ///
        /// specify the node external address and port.
        /// for the case of exposing the service to the outside,
        /// you need to manually specify the server external IP
        /// address and service listening port.
        ///
        external: SocketAddr,
        ///
        /// Idle timeout
        ///
        /// If no packet is received within the specified number of seconds, the
        /// connection will be closed to prevent resources from being occupied
        /// for a long time.
        #[serde(default = "Interface::idle_timeout")]
        idle_timeout: u32,
        ///
        /// Maximum Transmission Unit (MTU) size for network packets.
        ///
        #[serde(default = "Interface::mtu")]
        mtu: usize,
    },
}

impl Interface {
    fn mtu() -> usize {
        1500
    }

    fn idle_timeout() -> u32 {
        20
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Server {
    ///
    /// Port range, the maximum range is 65535 - 49152.
    ///
    #[serde(default = "Server::port_range")]
    pub port_range: PortRange,
    ///
    /// Maximum number of threads the TURN server can use.
    ///
    #[serde(default = "Server::max_threads")]
    pub max_threads: usize,
    ///
    /// turn server realm
    ///
    /// specify the domain where the server is located.
    /// for a single node, this configuration is fixed,
    /// but each node can be configured as a different domain.
    /// this is a good idea to divide the nodes by namespace.
    ///
    #[serde(default = "Server::realm")]
    pub realm: String,
    ///
    /// turn server listen interfaces
    ///
    /// The address and port to which the UDP Server is bound. Multiple
    /// addresses can be bound at the same time. The binding address supports
    /// ipv4 and ipv6.
    ///
    #[serde(default)]
    pub interfaces: Vec<Interface>,
}

impl Server {
    pub fn get_external_addresses(&self) -> Vec<SocketAddr> {
        self.interfaces
            .iter()
            .map(|item| match item {
                Interface::Tcp { external, .. } => *external,
                Interface::Udp { external, .. } => *external,
            })
            .collect()
    }
}

impl Server {
    fn realm() -> String {
        "localhost".to_string()
    }

    fn port_range() -> PortRange {
        PortRange::default()
    }

    fn max_threads() -> usize {
        num_cpus::get()
    }
}

impl Default for Server {
    fn default() -> Self {
        Self {
            realm: Self::realm(),
            interfaces: Default::default(),
            port_range: Self::port_range(),
            max_threads: Self::max_threads(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Hooks {
    #[serde(default = "Hooks::max_channel_size")]
    pub max_channel_size: usize,
    pub endpoint: String,
    #[serde(default)]
    pub ssl: Option<Ssl>,
    #[serde(default = "Hooks::timeout")]
    pub timeout: u32,
}

impl Hooks {
    fn max_channel_size() -> usize {
        1024
    }

    fn timeout() -> u32 {
        5
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Api {
    ///
    /// rpc server listen
    ///
    /// This option specifies the rpc server binding address used to control
    /// the turn server.
    ///
    #[serde(default = "Api::bind")]
    pub listen: SocketAddr,
    #[serde(default)]
    pub ssl: Option<Ssl>,
    #[serde(default = "Api::timeout")]
    pub timeout: u32,
}

impl Api {
    fn bind() -> SocketAddr {
        "127.0.0.1:3000".parse().unwrap()
    }

    fn timeout() -> u32 {
        5
    }
}

impl Default for Api {
    fn default() -> Self {
        Self {
            timeout: Self::timeout(),
            listen: Self::bind(),
            ssl: None,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Prometheus {
    ///
    /// prometheus server listen
    ///
    /// This option specifies the prometheus server binding address used to expose
    /// the metrics.
    ///
    #[serde(default = "Prometheus::bind")]
    pub listen: SocketAddr,
    ///
    /// ssl configuration
    ///
    /// This option specifies the ssl configuration for the prometheus server.
    ///
    #[serde(default)]
    pub ssl: Option<Ssl>,
}

impl Prometheus {
    fn bind() -> SocketAddr {
        "127.0.0.1:9090".parse().unwrap()
    }
}

impl Default for Prometheus {
    fn default() -> Self {
        Self {
            listen: Self::bind(),
            ssl: None,
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "trace" => Self::Trace,
            "debug" => Self::Debug,
            "info" => Self::Info,
            "warn" => Self::Warn,
            "error" => Self::Error,
            _ => return Err(format!("unknown log level: {value}")),
        })
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl LogLevel {
    pub fn as_level(&self) -> log::Level {
        match *self {
            Self::Error => log::Level::Error,
            Self::Debug => log::Level::Debug,
            Self::Trace => log::Level::Trace,
            Self::Warn => log::Level::Warn,
            Self::Info => log::Level::Info,
        }
    }
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Log {
    ///
    /// log level
    ///
    /// An enum representing the available verbosity levels of the logger.
    ///
    #[serde(default)]
    pub level: LogLevel,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Auth {
    ///
    /// static user password
    ///
    /// This option can be used to specify the static identity authentication
    /// information used by the turn server for verification.
    ///
    /// Note: this is a high-priority authentication method, turn The server will
    /// try to use static authentication first, and then use external control
    /// service authentication.
    ///
    #[serde(default)]
    pub static_credentials: HashMap<String, String>,
    ///
    /// Static authentication key value (string) that applies only to the TURN
    /// REST API.
    ///
    /// If set, the turn server will not request external services via the HTTP
    /// Hooks API to obtain the key.
    ///
    pub static_auth_secret: Option<String>,
    #[serde(default)]
    pub enable_hooks_auth: bool,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub server: Server,
    #[serde(default)]
    pub api: Option<Api>,
    #[serde(default)]
    pub prometheus: Option<Prometheus>,
    #[serde(default)]
    pub hooks: Option<Hooks>,
    #[serde(default)]
    pub log: Log,
    #[serde(default)]
    pub auth: Auth,
}

#[derive(Parser, Debug)]
#[command(
    about = env!("CARGO_PKG_DESCRIPTION"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
)]
struct Cli {
    ///
    /// Specify the configuration file path
    ///
    /// Example: turn-server --config /etc/turn-rs/config.toml
    ///
    #[arg(long, short)]
    config: String,
}

impl Config {
    ///
    /// Load configure from config file and command line parameters.
    ///
    /// Load command line parameters, if the configuration file path is specified,
    /// the configuration is read from the configuration file, otherwise the
    /// default configuration is used.
    ///
    pub fn load() -> Result<Self> {
        Ok(toml::from_str::<Self>(&read_to_string(
            &Cli::parse().config,
        )?)?)
    }
}
