use std::{collections::HashMap, fs::read_to_string, net::SocketAddr, ops::Range, str::FromStr};

use anyhow::anyhow;
use clap::Parser;
use serde::{Deserialize, Serialize};

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
    /// SSL configuration
    ///
    pub ssl: Option<Ssl>,
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct Api {
    ///
    /// api server listen
    ///
    /// This option specifies the http server binding address used to control
    /// the turn server.
    ///
    #[serde(default = "Api::bind")]
    pub listen: SocketAddr,
    ///
    /// HTTP response headers
    ///
    /// Used to customize, add, or override HTTP response headers for the API
    /// service, such as setting cross-origin (CORS) related parameters.
    ///
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub ssl: Option<Ssl>,
}

impl Api {
    fn bind() -> SocketAddr {
        "127.0.0.1:3000".parse().unwrap()
    }
}

impl Default for Api {
    fn default() -> Self {
        Self {
            headers: Default::default(),
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

#[derive(Deserialize, Debug, Default)]
pub struct Log {
    ///
    /// log level
    ///
    /// An enum representing the available verbosity levels of the logger.
    ///
    #[serde(default)]
    pub level: LogLevel,
}

#[derive(Deserialize, Debug, Default)]
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
}

#[derive(Deserialize, Debug, Clone)]
pub struct Runtime {
    ///
    /// Port range, the maximum range is 65535 - 49152.
    ///
    #[serde(default = "Runtime::port_range")]
    pub port_range: Range<u16>,
    ///
    /// Maximum number of threads the TURN server can use.
    ///
    #[serde(default = "Runtime::max_threads")]
    pub max_threads: usize,
    ///
    /// Maximum Transmission Unit (MTU) size for network packets.
    ///
    #[serde(default = "Runtime::mtu")]
    pub mtu: usize,
}

impl Runtime {
    fn port_range() -> Range<u16> {
        49152..65535
    }

    fn max_threads() -> usize {
        num_cpus::get()
    }

    fn mtu() -> usize {
        1500
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            port_range: Self::port_range(),
            max_threads: Self::max_threads(),
            mtu: Self::mtu(),
        }
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct Config {
    #[serde(default)]
    pub turn: Turn,
    #[serde(default)]
    pub api: Api,
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
