use std::{collections::HashMap, fs::read_to_string, net::SocketAddr, str::FromStr};

use anyhow::anyhow;
use clap::Parser;
use serde::{Deserialize, Serialize};
use service::session::ports::PortRange;

#[repr(C)]
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Tcp = 0,
    Udp = 1,
}

impl FromStr for Transport {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "udp" => Self::Udp,
            "tcp" => Self::Tcp,
            _ => return Err(anyhow!("unknown transport: {value}")),
        })
    }
}

/// SSL configuration
#[derive(Deserialize, Serialize, Debug, Clone)]
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
pub struct Interface {
    pub transport: Transport,
    pub listen: SocketAddr,
    ///
    /// external address
    ///
    /// specify the node external address and port.
    /// for the case of exposing the service to the outside,
    /// you need to manually specify the server external IP
    /// address and service listening port.
    ///
    pub external: SocketAddr,
    ///
    /// Maximum Transmission Unit (MTU) size for network packets.
    ///
    #[serde(default = "Interface::mtu")]
    pub mtu: usize,
    ///
    /// SSL configuration
    ///
    #[serde(default)]
    pub ssl: Option<Ssl>,
}

impl Interface {
    fn mtu() -> usize {
        1500
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Turn {
    ///
    /// turn server realm
    ///
    /// specify the domain where the server is located.
    /// for a single node, this configuration is fixed,
    /// but each node can be configured as a different domain.
    /// this is a good idea to divide the nodes by namespace.
    ///
    #[serde(default = "Turn::realm")]
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

impl Turn {
    pub fn get_externals(&self) -> Vec<SocketAddr> {
        self.interfaces.iter().map(|item| item.external).collect()
    }
}

impl Turn {
    fn realm() -> String {
        "localhost".to_string()
    }
}

impl Default for Turn {
    fn default() -> Self {
        Self {
            realm: Self::realm(),
            interfaces: Default::default(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Hooks {
    #[serde(default = "Hooks::max_channel_size")]
    pub max_channel_size: usize,
    pub endpoint: String,
    #[serde(default)]
    pub ssl: Option<Ssl>,
}

impl Hooks {
    fn max_channel_size() -> usize {
        1024
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Rpc {
    ///
    /// rpc server listen
    ///
    /// This option specifies the rpc server binding address used to control
    /// the turn server.
    ///
    #[serde(default = "Rpc::bind")]
    pub listen: SocketAddr,
    #[serde(default)]
    pub hooks: Option<Hooks>,
    #[serde(default)]
    pub ssl: Option<Ssl>,
}

impl Rpc {
    fn bind() -> SocketAddr {
        "127.0.0.1:3000".parse().unwrap()
    }
}

impl Default for Rpc {
    fn default() -> Self {
        Self {
            listen: Self::bind(),
            hooks: None,
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

#[derive(Deserialize, Debug, Clone)]
pub struct Runtime {
    ///
    /// Port range, the maximum range is 65535 - 49152.
    ///
    #[serde(default = "Runtime::port_range")]
    pub port_range: PortRange,
    ///
    /// Maximum number of threads the TURN server can use.
    ///
    #[serde(default = "Runtime::max_threads")]
    pub max_threads: usize,
}

impl Runtime {
    fn port_range() -> PortRange {
        PortRange::default()
    }

    fn max_threads() -> usize {
        num_cpus::get()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            port_range: Self::port_range(),
            max_threads: Self::max_threads(),
        }
    }
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub turn: Turn,
    #[serde(default)]
    pub rpc: Rpc,
    #[serde(default)]
    pub log: Log,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub runtime: Runtime,
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
    config: Option<String>,
}

impl Config {
    ///
    /// Load configure from config file and command line parameters.
    ///
    /// Load command line parameters, if the configuration file path is specified,
    /// the configuration is read from the configuration file, otherwise the
    /// default configuration is used.
    ///
    pub fn load() -> anyhow::Result<Self> {
        Ok(serde_json5::from_str::<Self>(
            &Cli::parse()
                .config
                .and_then(|path| read_to_string(path).ok())
                .unwrap_or("".to_string()),
        )?)
    }
}
