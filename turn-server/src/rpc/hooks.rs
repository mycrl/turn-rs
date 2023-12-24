use super::proto::{
    hooks_client::HooksClient, AbortRequest, AllocatedRequest, BindingRequest, ChannelBindRequest,
    CreatePermissionRequest, GetPasswordRequest, RefreshRequest,
};

use crate::config::Config;
use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use tokio::sync::Mutex;
use tonic::transport::Channel;

/// hooks
///
/// The hooks is used for the turn server to send requests to the
/// outside and notify or obtain information necessary for operation.
pub struct Hooks {
    client: Option<Mutex<HooksClient<Channel>>>,
    config: Arc<Config>,
}

impl Hooks {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            client: if let Some(uri) = config.hooks.bind.clone() {
                HooksClient::connect(uri).await.ok().map(Mutex::new)
            } else {
                None
            },
        })
    }

    /// get key by username.
    ///
    /// It should be noted that by default, it will first check whether
    /// the current user's authentication information has been included in
    /// the static authentication list. If it has been included, it will
    /// directly return the key in the static authentication information.
    /// If it is not included, it will request an external service to
    /// obtain the key.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message GetPasswordRequest {
    ///     string addr = 1;
    ///     string name = 2;
    /// }
    ///
    /// message GetPasswordReply {
    ///     string password = 1;
    /// }
    ///
    /// rpc GetPassword (GetPasswordRequest) returns (GetPasswordReply);
    /// ```
    pub async fn get_password(&self, addr: &SocketAddr, name: &str) -> Option<String> {
        if let Some(v) = self.config.auth.get(name) {
            return Some(v.clone());
        }

        Some(
            self.client
                .as_ref()?
                .lock()
                .await
                .get_password(GetPasswordRequest {
                    addr: addr.to_string(),
                    name: name.to_string(),
                })
                .await
                .ok()?
                .into_inner()
                .password,
        )
    }

    /// Request Port Assignment event.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message AllocatedRequest {
    ///     string addr = 1;
    ///     string name = 2;
    ///     uint32 port = 3;
    /// }
    ///
    /// rpc Allocated (AllocatedRequest) returns (google.protobuf.Empty);
    /// ```
    pub async fn allocated(&self, addr: &SocketAddr, name: &str, port: u16) {
        if self.config.hooks.events.iter().any(|k| k == "allocated") {
            if let Some(client) = self.client.as_ref() {
                let _ = client
                    .lock()
                    .await
                    .allocated(AllocatedRequest {
                        addr: addr.to_string(),
                        name: name.to_string(),
                        port: port as u32,
                    })
                    .await;
            }
        }
    }

    /// Binding request event.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message BindingRequest {
    ///     string addr = 1;
    /// }
    ///
    /// rpc Binding (BindingRequest) returns (google.protobuf.Empty);
    /// ```
    pub async fn binding(&self, addr: &SocketAddr) {
        if self.config.hooks.events.iter().any(|k| k == "binding") {
            if let Some(client) = self.client.as_ref() {
                let _ = client
                    .lock()
                    .await
                    .binding(BindingRequest {
                        addr: addr.to_string(),
                    })
                    .await;
            }
        }
    }

    /// Request Binding Channel event.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message ChannelBindRequest {
    ///     string addr = 1;
    ///     string name = 2;
    ///     uint32 number = 3;
    /// }
    ///
    /// rpc ChannelBind (ChannelBindRequest) returns (google.protobuf.Empty);
    /// ```
    pub async fn channel_bind(&self, addr: &SocketAddr, name: &str, number: u16) {
        if self.config.hooks.events.iter().any(|k| k == "channel_bind") {
            if let Some(client) = self.client.as_ref() {
                let _ = client
                    .lock()
                    .await
                    .channel_bind(ChannelBindRequest {
                        addr: addr.to_string(),
                        name: name.to_string(),
                        number: number as u32,
                    })
                    .await;
            }
        }
    }

    /// Creating Permission event.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message CreatePermissionRequest {
    ///     string addr = 1;
    ///     string name = 2;
    ///     string relay = 3;
    /// }
    ///
    /// rpc CreatePermission (CreatePermissionRequest) returns (google.protobuf.Empty);
    /// ```
    pub async fn create_permission(&self, addr: &SocketAddr, name: &str, relay: &SocketAddr) {
        if self
            .config
            .hooks
            .events
            .iter()
            .any(|k| k == "create_permission")
        {
            if let Some(client) = self.client.as_ref() {
                let _ = client
                    .lock()
                    .await
                    .create_permission(CreatePermissionRequest {
                        addr: addr.to_string(),
                        name: name.to_string(),
                        relay: relay.to_string(),
                    })
                    .await;
            }
        }
    }

    /// Refresh Lifecycle event.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message RefreshRequest {
    ///     string addr = 1;
    ///     string name = 2;
    ///     uint32 time = 3;
    /// }
    ///
    /// rpc Refresh (RefreshRequest) returns (google.protobuf.Empty);
    /// ```
    pub async fn refresh(&self, addr: &SocketAddr, name: &str, time: u32) {
        if self.config.hooks.events.iter().any(|k| k == "refresh") {
            if let Some(client) = self.client.as_ref() {
                let _ = client
                    .lock()
                    .await
                    .refresh(RefreshRequest {
                        addr: addr.to_string(),
                        name: name.to_string(),
                        time,
                    })
                    .await;
            }
        }
    }

    /// end-of-session event.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message AbortRequest {
    ///     string addr = 1;
    ///     string name = 2;
    /// }
    ///
    /// rpc Abort (AbortRequest) returns (google.protobuf.Empty);
    /// ```
    pub async fn abort(&self, addr: &SocketAddr, name: &str) {
        if self.config.hooks.events.iter().any(|k| k == "abort") {
            if let Some(client) = self.client.as_ref() {
                let _ = client
                    .lock()
                    .await
                    .abort(AbortRequest {
                        addr: addr.to_string(),
                        name: name.to_string(),
                    })
                    .await;
            }
        }
    }
}
