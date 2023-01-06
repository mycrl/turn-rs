use clap::Parser;
use serde::*;
use std::{
    fs::read_to_string,
    net::SocketAddr,
    collections::HashMap,
};

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

    /// external address
    ///
    /// specify the node external address and port.
    /// for the case of exposing the service to the outside,
    /// you need to manually specify the server external IP
    /// address and service listening port.
    #[serde(default = "Turn::external")]
    pub external: SocketAddr,

    /// turn server listen address
    ///
    /// the address and port bound by UDP Server.
    /// currently, it does not support binding multiple
    /// addresses at the same time. the bound address
    /// supports ipv4 and ipv6.
    #[serde(default = "Turn::listen")]
    pub listen: SocketAddr,

    /// thread number
    ///
    /// by default, the thread pool is used to process UDP packets.
    /// because UDP uses SysCall to ensure concurrency security,
    /// using multiple threads may not bring a very significant
    /// performance improvement, but setting the number of CPU
    /// cores can process data to the greatest extent package.
    #[serde(default = "num_cpus::get")]
    pub threads: usize,
}

impl Turn {
    fn realm() -> String {
        "localhost".to_string()
    }

    fn external() -> SocketAddr {
        "127.0.0.1:3478".parse().unwrap()
    }

    fn listen() -> SocketAddr {
        "127.0.0.1:3478".parse().unwrap()
    }
}

impl Default for Turn {
    fn default() -> Self {
        Self {
            realm: Self::realm(),
            external: Self::external(),
            listen: Self::listen(),
            threads: num_cpus::get(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Controller {
    /// controller bind
    ///
    /// This option specifies the http server binding address used to control
    /// the turn server.
    ///
    /// Warn: This http server does not contain any means of authentication,
    /// and sensitive information and dangerous operations can be obtained
    /// through this service, please do not expose it directly to an unsafe
    /// environment.
    #[serde(default = "Controller::listen")]
    pub listen: SocketAddr,

    /// Set the value of the Access-Control-Allow-Origin header.
    ///
    /// Access-Control-Allow-Origin is a header request that states whether the
    /// response is shared with requesting code.
    #[serde(default = "Controller::allow_origin")]
    pub allow_origin: String,
}

impl Controller {
    fn listen() -> SocketAddr {
        "127.0.0.1:3000".parse().unwrap()
    }

    fn allow_origin() -> String {
        "*".to_string()
    }
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            listen: Self::listen(),
            allow_origin: Self::allow_origin(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Hooks {
    /// hooks bind uri
    ///
    /// This option is used to specify the http address of the hooks service.
    ///
    /// Warn: This http server does not contain any means of authentication,
    /// and sensitive information and dangerous operations can be obtained
    /// through this service, please do not expose it directly to an unsafe
    /// environment.
    #[serde(default = "Hooks::bind")]
    pub bind: String,

    /// list of events followed by hooks
    ///
    /// event list: "allocated", "binding", "channel_bind",
    /// "create_permission", "refresh", "abort".
    #[serde(default = "Hooks::sub_events")]
    pub sub_events: Vec<String>,
}

impl Hooks {
    fn bind() -> String {
        "http://localhost:8080".to_string()
    }

    fn sub_events() -> Vec<String> {
        vec![]
    }
}

impl Default for Hooks {
    fn default() -> Self {
        Self {
            bind: Self::bind(),
            sub_events: Self::sub_events(),
        }
    }
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    pub turn: Turn,
    #[serde(default)]
    pub controller: Controller,
    #[serde(default)]
    pub hooks: Hooks,
    #[serde(default)]
    pub log: Log,

    /// static user password
    ///
    /// This option can be used to specify the
    /// static identity authentication information used by the turn server for
    /// verification. Note: this is a high-priority authentication method, turn
    /// The server will try to use static authentication first, and then use
    /// external control service authentication.
    #[serde(default)]
    pub auth: HashMap<String, String>,
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
    pub fn load() -> Self {
        let cfg_str = Cli::parse()
            .config
            .map(|path| read_to_string(path).ok())
            .flatten()
            .unwrap_or("".to_string());
        toml::from_str(&cfg_str).expect("read config file failed!")
    }
}
