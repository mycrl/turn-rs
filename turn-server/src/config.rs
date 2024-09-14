use std::{collections::HashMap, fs::read_to_string, net::SocketAddr};

use clap::Parser;
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    TCP = 0,
    UDP = 1,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Interface {
    pub transport: Transport,
    /// turn server listen address
    pub bind: SocketAddr,
    /// external address
    ///
    /// specify the node external address and port.
    /// for the case of exposing the service to the outside,
    /// you need to manually specify the server external IP
    /// address and service listening port.
    pub external: SocketAddr,
}

#[derive(Deserialize, Debug)]
pub struct Turn {
    /// turn server realm
    ///
    /// specify the domain where the server is located.
    /// for a single node, this configuration is fixed,
    /// but each node can be configured as a different domain.
    /// this is a good idea to divide the nodes by namespace.
    #[serde(default = "Turn::realm")]
    pub realm: String,

    /// turn server listen interfaces
    ///
    /// The address and port to which the UDP Server is bound. Multiple
    /// addresses can be bound at the same time. The binding address supports
    /// ipv4 and ipv6.
    #[serde(default = "Turn::interfaces")]
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

    fn interfaces() -> Vec<Interface> {
        vec![]
    }
}

impl Default for Turn {
    fn default() -> Self {
        Self {
            realm: Self::realm(),
            interfaces: Self::interfaces(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Api {
    /// api bind
    ///
    /// This option specifies the http server binding address used to control
    /// the turn server.
    ///
    /// Warn: This http server does not contain any means of authentication,
    /// and sensitive information and dangerous operations can be obtained
    /// through this service, please do not expose it directly to an unsafe
    /// environment.
    #[serde(default = "Api::bind")]
    pub bind: SocketAddr,
    /// hooks server url
    ///
    /// This option is used to specify the http address of the hooks service.
    ///
    /// Warn: This http server does not contain any means of authentication,
    /// and sensitive information and dangerous operations can be obtained
    /// through this service, please do not expose it directly to an unsafe
    /// environment.
    pub hooks: Option<String>,
}

impl Api {
    fn bind() -> SocketAddr {
        "127.0.0.1:3000".parse().unwrap()
    }
}

impl Default for Api {
    fn default() -> Self {
        Self {
            hooks: None,
            bind: Self::bind(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
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
    /// log level
    ///
    /// An enum representing the available verbosity levels of the logger.
    #[serde(default)]
    pub level: LogLevel,
}

#[derive(Deserialize, Debug, Default)]
pub struct Auth {
    /// static user password
    ///
    /// This option can be used to specify the
    /// static identity authentication information used by the turn server for
    /// verification. Note: this is a high-priority authentication method, turn
    /// The server will try to use static authentication first, and then use
    /// external control service authentication.
    #[serde(default)]
    pub static_credentials: HashMap<String, String>,
    /// Static authentication key value (string) that applies only to the TURN
    /// REST API.
    ///
    /// If set, the turn server will not request external services via the HTTP
    /// Hooks API to obtain the key.
    pub static_auth_secret: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    pub turn: Turn,
    #[serde(default)]
    pub api: Api,
    #[serde(default)]
    pub log: Log,
    #[serde(default)]
    pub auth: Auth,
}

#[derive(Parser)]
#[command(
    about = env!("CARGO_PKG_DESCRIPTION"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
)]
struct Cli {
    /// specify the configuration file path.
    #[arg(long)]
    config: Option<String>,
}

impl Config {
    /// Load command line parameters, if the configuration file path is
    /// specified, the configuration is read from the configuration file,
    /// otherwise the default configuration is used.
    pub fn load() -> anyhow::Result<Self> {
        Ok(toml::from_str(
            &Cli::parse()
                .config
                .and_then(|path| read_to_string(path).ok())
                .unwrap_or("".to_string()),
        )?)
    }
}
