use crate::config::Config;
use clap::ValueEnum;
use serde::*;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    fs,
};

use anyhow::{
    Result,
    anyhow,
};

/// hooks events kind
#[derive(ValueEnum)]
#[derive(Debug, Clone, PartialEq)]
pub enum Events {
    Allocated,
    Binding,
    ChannelBind,
    CreatePermission,
    Refresh,
    Abort,
}

impl Events {
    fn to_str(&self) -> &'static str {
        match self {
            Self::Allocated => "allocated",
            Self::Binding => "binding",
            Self::ChannelBind => "channel_bind",
            Self::CreatePermission => "create_permission",
            Self::Refresh => "refresh",
            Self::Abort => "abort",
        }
    }
}

#[rustfmt::skip]
#[derive(Serialize)]
pub enum EventsBody<'a> {
    /// allocate request
    Allocated {
        addr: &'a SocketAddr,
        name: &'a str,
        port: u16,
    },
    /// binding request
    Binding { 
        addr: &'a SocketAddr 
    },
    /// channel binding request
    ChannelBind {
        addr: &'a SocketAddr,
        name: &'a str,
        number: u16,
    },
    /// create permission request
    CreatePermission {
        addr: &'a SocketAddr,
        name: &'a str,
        relay: &'a SocketAddr,
    },
    /// refresh request
    Refresh {
        addr: &'a SocketAddr,
        name: &'a str,
        time: u32,
    },
    /// node exit
    Abort { 
        addr: &'a SocketAddr, 
        name: &'a str 
    },
}

/// web hooks
///
/// The web hooks is used for the turn server to send requests to the
/// outside and notify or obtain information necessary for operation.
pub struct Hooks {
    static_certs: HashMap<String, String>,
    client: reqwest::Client,
    config: Arc<Config>,
}

impl Hooks {
    fn hooks(res: reqwest::Response) -> Result<reqwest::Response> {
        log::info!("hooks response: {:?}", res);
        (res.status() == 200)
            .then(|| res)
            .ok_or_else(|| anyhow!("request failed!"))
    }

    /// Create an web hooks
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = Config::new()
    /// // let ext_ctr = Hooks::new(config);
    /// ```
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            static_certs: config
                .cert_file
                .as_ref()
                .map(|f| fs::read_to_string(&f).unwrap_or("".to_string()))
                .map(|s| toml::from_str(&s).unwrap())
                .unwrap_or_else(|| HashMap::new()),
            client: reqwest::Client::new(),
            config,
        }
    }

    /// request hooks authentication.
    ///
    /// This interface will first try to find the internal static certificate
    /// table, if not found, then request the web interface for
    /// authentication.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = Config::new()
    /// let ext_ctr = Hooks::new(config);
    ///
    /// let addr = "127.0.0.1:8080".parse().unwrap();
    /// // let key = ext_ctr.auth(&addr, "test").await?;
    /// ```
    pub async fn auth(&self, addr: &SocketAddr, name: &str) -> Result<String> {
        if let Some(v) = self.static_certs.get(name) {
            return Ok(v.clone());
        }

        Ok(Self::hooks(
            self.client
                .get(format!(
                    "{}/auth?addr={}&name={}",
                    self.config.hooks_uri, addr, name
                ))
                .send()
                .await?,
        )?
        .text()
        .await?)
    }

    /// push event
    ///
    /// Only subscribed events are pushed, other events are ignored.
    ///
    /// TODO: This method will not wait for the send to succeed, and will
    /// complete regardless of success or failure.
    pub fn events(&self, kind: Events, body: &EventsBody<'_>) {
        if self.config.hooks_events.contains(&kind) {
            return;
        }

        let _ = self
            .client
            .put(format!(
                "{}/events?kind={}",
                self.config.hooks_uri,
                kind.to_str()
            ))
            .json(body)
            .send();
    }
}
