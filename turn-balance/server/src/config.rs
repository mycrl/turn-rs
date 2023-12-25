use std::net::SocketAddr;

use clap::Parser;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Net {
    pub bind: SocketAddr,
}

#[derive(Deserialize, Debug)]
pub struct Cluster {
    pub superiors: Option<SocketAddr>,
    pub nodes: Vec<SocketAddr>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub net: Net,
    pub cluster: Cluster,
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
