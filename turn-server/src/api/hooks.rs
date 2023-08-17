use super::payload::Events;
use crate::config::Config;

use std::{net::SocketAddr, sync::Arc};

use anyhow::{anyhow, Result};

/// web hooks
///
/// The web hooks is used for the turn server to send requests to the
/// outside and notify or obtain information necessary for operation.
pub struct Hooks {
    client: reqwest::Client,
    config: Arc<Config>,
}

impl Hooks {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    pub async fn auth(&self, addr: &SocketAddr, name: &str) -> Result<String> {
        if let Some(v) = self.config.auth.get(name) {
            return Ok(v.clone());
        }

        let uri = match &self.config.hooks.bind {
            Some(h) => format!("{}/auth?addr={}&name={}", h, addr, name),
            None => return Err(anyhow!("auth failed!")),
        };

        let res = self.client.get(uri).send().await?;
        if res.status() != 200 {
            Err(anyhow!("request failed!"))
        } else {
            Ok(res.text().await?)
        }
    }

    pub fn on_events(&self, event: &Events<'_>) {
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
