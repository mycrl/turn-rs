use std::net::SocketAddr;

use clap::Parser;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Net {
    /// turn The address on which the balance server listens, for example
    /// `127.0.0.1:3001` or `0.0.0.0:3001` listens on all interfaces.
    #[serde(default = "Net::bind")]
    pub bind: SocketAddr,
}

impl Net {
    fn bind() -> SocketAddr {
        "127.0.0.1:3001".parse().unwrap()
    }
}

impl Default for Net {
    fn default() -> Self {
        Self { bind: Self::bind() }
    }
}

#[derive(Deserialize, Debug)]
pub struct Cluster {
    /// In the network topology, if there is a superior turn balance server, it
    /// needs to be specified here because the superior server needs to know if
    /// the current server is online, which needs to be realized by the current
    /// server actively sending udp heartbeat packets.
    pub superiors: Option<SocketAddr>,
    #[serde(default = "Cluster::nodes")]
    /// The subordinate nodes of the current turn balance server, either turn
    /// server or the same turn balance server, please note that this is a list
    /// of nodes and you can specify more than one server at the same time.
    pub nodes: Vec<SocketAddr>,
}

impl Cluster {
    fn nodes() -> Vec<SocketAddr> {
        vec![]
    }
}

impl Default for Cluster {
    fn default() -> Self {
        Self {
            superiors: None,
            nodes: Self::nodes(),
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
pub struct Turn {
    /// How turn balance server and turn server belong to the same node and you
    /// deploy turn balance and turn server separately and individually, here
    /// you need to specify the turn server listening address that you expect to
    /// report to the client, which allows the client to connect to the turn
    /// server.
    #[serde(default)]
    pub bind: Option<SocketAddr>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    pub net: Net,
    #[serde(default)]
    pub cluster: Cluster,
    #[serde(default)]
    pub log: Log,
    #[serde(default)]
    pub turn: Turn,
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
                .and_then(|path| std::fs::read_to_string(path).ok())
                .unwrap_or("".to_string()),
        )?)
    }
}
