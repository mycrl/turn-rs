use std::{collections::HashMap, fs::read_to_string, net::SocketAddr, str::FromStr};

use anyhow::anyhow;
use clap::Parser;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    TCP = 0,
    UDP = 1,
}

impl FromStr for Transport {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "udp" => Self::UDP,
            "tcp" => Self::TCP,
            _ => return Err(anyhow!("unknown transport: {value}")),
        })
    }
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

impl FromStr for Interface {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (transport, addrs) = s
            .split('@')
            .collect_tuple()
            .ok_or_else(|| anyhow!("invalid interface transport: {}", s))?;

        let (bind, external) = addrs
            .split('/')
            .collect_tuple()
            .ok_or_else(|| anyhow!("invalid interface address: {}", s))?;

        Ok(Interface {
            external: external.parse::<SocketAddr>()?,
            bind: bind.parse::<SocketAddr>()?,
            transport: transport.parse()?,
        })
    }
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

#[derive(Parser, Debug)]
#[command(
    about = env!("CARGO_PKG_DESCRIPTION"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
)]
struct Cli {
    /// Specify the configuration file path
    ///
    /// Example: --config /etc/turn-rs/config.toml
    #[arg(long, short)]
    config: Option<String>,
    /// Static user password
    ///
    /// Example: --auth-static-credentials test=test
    #[arg(long, value_parser = Cli::parse_credential)]
    auth_static_credentials: Option<Vec<(String, String)>>,
    /// Static authentication key value (string) that applies only to the TURN
    /// REST API
    #[arg(long)]
    auth_static_auth_secret: Option<String>,
    /// An enum representing the available verbosity levels of the logger
    #[arg(
        long,
        value_parser = clap::value_parser!(LogLevel),
    )]
    log_level: Option<LogLevel>,
    /// This option specifies the http server binding address used to control
    /// the turn server
    #[arg(long)]
    api_bind: Option<SocketAddr>,
    /// This option is used to specify the http address of the hooks service
    ///
    /// Example: --api-hooks http://localhost:8080/turn
    #[arg(long)]
    api_hooks: Option<String>,
    /// TURN server realm
    #[arg(long)]
    turn_realm: Option<String>,
    /// TURN server listen interfaces
    ///
    /// Example: --turn-interfaces udp@127.0.0.1:3478/127.0.0.1:3478
    #[arg(long)]
    turn_interfaces: Option<Vec<Interface>>,
}

impl Cli {
    // [username]:[password]
    fn parse_credential(s: &str) -> Result<(String, String), anyhow::Error> {
        let (username, password) = s
            .split('=')
            .collect_tuple()
            .ok_or_else(|| anyhow!("invalid credential str: {}", s))?;
        Ok((username.to_string(), password.to_string()))
    }
}

impl Config {
    /// Load command line parameters, if the configuration file path is
    /// specified, the configuration is read from the configuration file,
    /// otherwise the default configuration is used.
    pub fn load() -> anyhow::Result<Self> {
        let cli = Cli::parse();
        let mut config = toml::from_str::<Self>(
            &cli.config
                .and_then(|path| read_to_string(path).ok())
                .unwrap_or("".to_string()),
        )?;

        // Command line arguments have a high priority and override configuration file
        // options; here they are used to replace the configuration parsed out of the
        // configuration file.
        {
            if let Some(credentials) = cli.auth_static_credentials {
                for (k, v) in credentials {
                    config.auth.static_credentials.insert(k, v);
                }
            }

            if let Some(secret) = cli.auth_static_auth_secret {
                config.auth.static_auth_secret.replace(secret);
            }

            if let Some(level) = cli.log_level {
                config.log.level = level;
            }

            if let Some(bind) = cli.api_bind {
                config.api.bind = bind;
            }

            if let Some(hooks) = cli.api_hooks {
                config.api.hooks.replace(hooks);
            }

            if let Some(realm) = cli.turn_realm {
                config.turn.realm = realm;
            }

            if let Some(interfaces) = cli.turn_interfaces {
                for interface in interfaces {
                    config.turn.interfaces.push(interface);
                }
            }
        }

        Ok(config)
    }
}
