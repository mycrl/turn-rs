use std::time::Duration;

use crate::{
    codec::{crypto::Password, message::attributes::PasswordAlgorithm},
    config::Config,
    service::session::Identifier,
};

use anyhow::{Result, anyhow};
use tokio::sync::{
    Mutex,
    mpsc::{Sender, channel},
};

use tonic::{
    Request,
    transport::{Certificate, Channel, ClientTlsConfig},
};

use sdk::protos::{
    TurnAllocatedEvent, TurnChannelBindEvent, TurnCreatePermissionEvent, TurnDestroyEvent,
    TurnRefreshEvent, TurnRegisterRequest, turn_hooks_service_client::TurnHooksServiceClient,
};

impl Into<sdk::protos::Transport> for crate::service::Transport {
    fn into(self) -> sdk::protos::Transport {
        use sdk::protos::Transport;

        match self {
            Self::Udp => Transport::Udp,
            Self::Tcp => Transport::Tcp,
        }
    }
}

impl TryFrom<sdk::protos::Transport> for crate::service::Transport {
    type Error = anyhow::Error;

    fn try_from(value: sdk::protos::Transport) -> Result<Self, Self::Error> {
        use sdk::protos::Transport;

        match value {
            Transport::Udp => Ok(Self::Udp),
            Transport::Tcp => Ok(Self::Tcp),
            Transport::Unspecified => Err(anyhow!("transport is unspecified")),
        }
    }
}

impl Into<sdk::protos::PasswordAlgorithm> for crate::codec::message::attributes::PasswordAlgorithm {
    fn into(self) -> sdk::protos::PasswordAlgorithm {
        use sdk::protos::PasswordAlgorithm;

        match self {
            Self::Md5 => PasswordAlgorithm::Md5,
            Self::Sha256 => PasswordAlgorithm::Sha256,
        }
    }
}

impl Into<sdk::protos::Identifier> for Identifier {
    fn into(self) -> sdk::protos::Identifier {
        sdk::protos::Identifier {
            source: self.source.to_string(),
            external: self.external.to_string(),
            interface: self.interface.to_string(),
            transport: Into::<sdk::protos::Transport>::into(self.transport) as i32,
        }
    }
}

impl TryFrom<sdk::protos::Identifier> for crate::service::session::Identifier {
    type Error = anyhow::Error;

    fn try_from(value: sdk::protos::Identifier) -> Result<Self, Self::Error> {
        use crate::service::{Transport, session::Identifier};

        Ok(Identifier {
            source: value.source.parse()?,
            external: value.external.parse()?,
            interface: value.interface.parse()?,
            transport: Transport::try_from(sdk::protos::Transport::try_from(value.transport)?)?,
        })
    }
}

impl Into<sdk::protos::Interface> for &crate::service::InterfaceAddr {
    fn into(self) -> sdk::protos::Interface {
        sdk::protos::Interface {
            address: self.addr.to_string(),
            external: self.external.to_string(),
            transport: Into::<sdk::protos::Transport>::into(self.transport) as i32,
        }
    }
}

pub enum HooksEvent {
    Allocated(TurnAllocatedEvent),
    ChannelBind(TurnChannelBindEvent),
    CreatePermission(TurnCreatePermissionEvent),
    Refresh(TurnRefreshEvent),
    Destroy(TurnDestroyEvent),
}

struct RpcHooksServiceInner {
    event_channel: Sender<HooksEvent>,
    client: Mutex<TurnHooksServiceClient<Channel>>,
}

pub struct RpcHooksService(Option<RpcHooksServiceInner>);

impl RpcHooksService {
    pub async fn new(config: &Config) -> Result<Self> {
        if let Some(hooks) = &config.hooks {
            let (event_channel, mut rx) = channel(hooks.max_channel_size);
            let client = {
                let mut builder = Channel::builder(hooks.endpoint.as_str().try_into()?);

                builder = builder.timeout(Duration::from_secs(hooks.timeout as u64));

                if let Some(ssl) = &hooks.ssl {
                    builder = builder.tls_config(
                        ClientTlsConfig::new()
                            .ca_certificate(Certificate::from_pem(ssl.certificate_chain.clone()))
                            .domain_name(
                                url::Url::parse(&hooks.endpoint)?.domain().ok_or_else(|| {
                                    anyhow::anyhow!("Invalid hooks server domain")
                                })?,
                            ),
                    )?;
                }

                TurnHooksServiceClient::new(
                    builder
                        .connect_timeout(Duration::from_secs(5))
                        .timeout(Duration::from_secs(1))
                        .connect_lazy(),
                )
            };

            {
                let mut client = client.clone();

                tokio::spawn(async move {
                    while let Some(event) = rx.recv().await {
                        if match event {
                            HooksEvent::Allocated(event) => {
                                client.on_allocated_event(Request::new(event)).await
                            }
                            HooksEvent::ChannelBind(event) => {
                                client.on_channel_bind_event(Request::new(event)).await
                            }
                            HooksEvent::CreatePermission(event) => {
                                client.on_create_permission_event(Request::new(event)).await
                            }
                            HooksEvent::Refresh(event) => {
                                client.on_refresh_event(Request::new(event)).await
                            }
                            HooksEvent::Destroy(event) => {
                                client.on_destroy_event(Request::new(event)).await
                            }
                        }
                        .is_err()
                        {
                            break;
                        }
                    }
                });
            }

            log::info!("create hooks client, endpoint={}", hooks.endpoint);

            Ok(Self(Some(RpcHooksServiceInner {
                client: Mutex::new(client),
                event_channel,
            })))
        } else {
            Ok(Self(None))
        }
    }

    pub fn send_event(&self, event: HooksEvent) {
        if let Some(inner) = &self.0
            && !inner.event_channel.is_closed()
            && let Err(e) = inner.event_channel.try_send(event)
        {
            log::error!("Failed to send event to hooks server: {}", e);
        }
    }

    pub async fn register(
        &self,
        id: &Identifier,
        realm: &str,
        username: &str,
        algorithm: PasswordAlgorithm,
    ) -> Option<Password> {
        if let Some(inner) = &self.0 {
            use sdk::protos::PasswordAlgorithm;

            let algorithm: PasswordAlgorithm = algorithm.into();

            let password = inner
                .client
                .lock()
                .await
                .register(Request::new(TurnRegisterRequest {
                    id: Some(id.into()),
                    realm: realm.to_string(),
                    username: username.to_string(),
                    algorithm: algorithm as i32,
                }))
                .await
                .ok()?
                .into_inner()
                .password;

            return Some(match algorithm {
                PasswordAlgorithm::Md5 => Password::Md5(password.try_into().ok()?),
                PasswordAlgorithm::Sha256 => Password::Sha256(password.try_into().ok()?),
                PasswordAlgorithm::Unspecified => unreachable!(),
            });
        }

        None
    }
}
