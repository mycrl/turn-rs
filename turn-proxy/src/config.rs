use std::{
    fs::read_to_string,
    net::SocketAddr,
};

use clap::Parser;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Net {
    #[serde(default = "Net::bind")]
    pub bind: SocketAddr,
    #[serde(default = "Net::recon_delay")]
    pub recon_delay: u64,
}

impl Net {
    fn recon_delay() -> u64 {
        5000
    }

    fn bind() -> SocketAddr {
        "127.0.0.1:3479".parse().unwrap()
    }
}

impl Default for Net {
    fn default() -> Self {
        Self {
            recon_delay: Self::recon_delay(),
            bind: Self::bind(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Node {
    pub bind: SocketAddr,
    pub external: SocketAddr,
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

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub log: Log,
    #[serde(default)]
    pub net: Net,
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
                .map(|path| read_to_string(path).ok())
                .flatten()
                .unwrap_or("".to_string()),
        )?)
    }
}
