use std::net::SocketAddr;

use clap::Parser;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Net {
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
    pub superiors: Option<SocketAddr>,
    #[serde(default = "Cluster::nodes")]
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
