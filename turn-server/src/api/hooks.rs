use crate::config::Config;
use serde::*;
use std::{
    net::SocketAddr,
    sync::Arc,
};

use anyhow::{
    Result,
    anyhow,
};

#[rustfmt::skip]
#[derive(Serialize)]
pub enum Events<'a> {
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

impl Events<'_> {
    #[rustfmt::skip]
    const fn to_str(&self) -> &'static str {
        match *self {
            Self::Allocated {..} => "allocated",
            Self::Binding {..} => "binding",
            Self::ChannelBind {..} => "channel_bind",
            Self::CreatePermission {..} => "create_permission",
            Self::Refresh {..} => "refresh",
            Self::Abort {..} => "abort",
        }
    }
}

/// web hooks
///
/// The web hooks is used for the turn server to send requests to the
/// outside and notify or obtain information necessary for operation.
pub struct Hooks {
    client: reqwest::Client,
    config: Arc<Config>,
}

impl Hooks {
    fn hooks(res: reqwest::Response) -> Result<reqwest::Response> {
        log::info!("hooks response: {:?}", res);
        (res.status() == 200)
            .then_some(res)
            .ok_or_else(|| anyhow!("request failed!"))
    }

    /// Create an web hooks
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// // let ext_ctr = Hooks::new(config);
    /// ```
    pub fn new(config: Arc<Config>) -> Self {
        Self {
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
    /// ```ignore
    /// let config = Config::new()
    /// let ext_ctr = Hooks::new(config);
    ///
    /// let addr = "127.0.0.1:8080".parse().unwrap();
    /// // let key = ext_ctr.auth(&addr, "test").await?;
    /// ```
    pub async fn auth(&self, addr: &SocketAddr, name: &str) -> Result<String> {
        if let Some(v) = self.config.auth.get(name) {
            return Ok(v.clone());
        }

        let uri = match &self.config.hooks.bind {
            Some(h) => format!("{}/auth?addr={}&name={}", h, addr, name),
            None => return Err(anyhow!("auth failed!")),
        };

        Ok(Self::hooks(self.client.get(uri).send().await?)?
            .text()
            .await?)
    }

    /// push event
    ///
    /// Only subscribed events are pushed, other events are ignored.
    ///
    /// TODO: This method will not wait for the send to succeed, and will
    /// complete regardless of success or failure.
    pub fn events(&self, event: &Events<'_>) {
        let uri = match &self.config.hooks.bind {
            Some(h) => format!("{}/events?kind={}", h, event.to_str()),
            None => return,
        };

        if self
            .config
            .hooks
            .sub_events
            .iter()
            .any(|k| k == event.to_str())
        {
            drop(self.client.put(uri).json(event).send());
        }
    }
}
