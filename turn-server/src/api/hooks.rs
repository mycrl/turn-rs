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

    /// get key by username
    ///
    /// ```base
    /// curl -X GET [host]/password?addr=[ip addr]&name=[string]
    /// ```
    ///
    /// It should be noted that by default, it will first check whether
    /// the current user's authentication information has been included in
    /// the static authentication list. If it has been included, it will
    /// directly return the key in the static authentication information.
    /// If it is not included, it will request an external service to
    /// obtain the key.
    pub async fn get_password(&self, addr: &SocketAddr, name: &str) -> Result<String> {
        if let Some(v) = self.config.auth.get(name) {
            return Ok(v.clone());
        }

        let res = self
            .client
            .get(match &self.config.hooks.bind {
                Some(h) => format!("{}/password?addr={}&name={}", h, addr, name),
                None => return Err(anyhow!("auth failed!")),
            })
            .send()
            .await?;
        if res.status() != 200 {
            Err(anyhow!("request failed!"))
        } else {
            Ok(res.text().await?)
        }
    }

    /// push events to external services
    ///
    /// ```base
    /// curl -X GET [host]/events?kind=[kinds]
    /// ```
    ///
    /// Note: This request does not care whether the event is received
    /// externally, and the handler is abandoned after the request is made.
    pub fn on_events(&self, event: &Events<'_>) {
        if self
            .config
            .hooks
            .sub_events
            .iter()
            .any(|k| k == event.kind_name())
        {
            drop(
                self.client
                    .put(match &self.config.hooks.bind {
                        Some(h) => format!("{}/events?kind={}", h, event.kind_name()),
                        None => return,
                    })
                    .json(event)
                    .send(),
            );
        }
    }
}
