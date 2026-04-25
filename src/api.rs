use std::time::{Duration, Instant};

use crate::{
    Service,
    codec::{crypto::Password, message::attributes::PasswordAlgorithm},
    config::Config,
    service::session::{Identifier, Session},
    statistics::Statistics,
};

use anyhow::{Result, anyhow};
use tokio::sync::{
    Mutex,
    mpsc::{Sender, channel},
};

use tonic::{
    Request, Response, Status,
    transport::{Channel, Server},
};

#[cfg(feature = "ssl")]
use tonic::transport::{Certificate, ClientTlsConfig, Identity, ServerTlsConfig};

use sdk::protos::{
    BindAddress, GetTurnPasswordRequest, TurnAllocatedEvent, TurnChannelBindEvent,
    TurnCreatePermissionEvent, TurnDestroyEvent, TurnRefreshEvent, TurnServerInfo, TurnSession,
    TurnSessionStatistics,
    turn_hooks_service_client::TurnHooksServiceClient,
    turn_service_server::{TurnService, TurnServiceServer},
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

struct RpcService {
    config: Config,
    service: Service,
    statistics: Statistics,
    uptime: Instant,
}

#[tonic::async_trait]
impl TurnService for RpcService {
    async fn get_info(&self, _: Request<()>) -> Result<Response<TurnServerInfo>, Status> {
        Ok(Response::new(TurnServerInfo {
            software: crate::SOFTWARE.to_string(),
            uptime: self.uptime.elapsed().as_secs(),
            interfaces: self
                .config
                .server
                .get_interface_addrs()
                .iter()
                .map(|it| it.into())
                .collect(),
            port_capacity: self.config.server.port_range.size() as u32,
            port_allocated: self.service.get_session_manager().allocated() as u32,
        }))
    }

    async fn get_session(
        &self,
        request: Request<sdk::protos::Identifier>,
    ) -> Result<Response<TurnSession>, Status> {
        if let Some(Session::Authenticated {
            username,
            allocated_port,
            channel_relay_table,
            port_relay_table,
            expires,
            ..
        }) = self
            .service
            .get_session_manager()
            .get_session(
                &Identifier::try_from(request.into_inner())
                    .map_err(|e| Status::internal(e.to_string()))?,
            )
            .get_ref()
        {
            Ok(Response::new(TurnSession {
                username: username.to_string(),
                allocated_port: allocated_port.map(|p| p as i32),
                expires: *expires as i64,
                permissions: port_relay_table
                    .iter()
                    .map(|(k, v)| BindAddress {
                        key: *k as i32,
                        value: Some(v.clone().into()),
                    })
                    .collect(),
                channels: channel_relay_table
                    .iter()
                    .map(|(k, v)| BindAddress {
                        key: *k as i32,
                        value: Some(v.clone().into()),
                    })
                    .collect(),
            }))
        } else {
            Err(Status::not_found("Session not found"))
        }
    }

    async fn get_session_statistics(
        &self,
        request: Request<sdk::protos::Identifier>,
    ) -> Result<Response<TurnSessionStatistics>, Status> {
        if let Some(counts) = self.statistics.get(
            &Identifier::try_from(request.into_inner())
                .map_err(|e| Status::internal(e.to_string()))?,
        ) {
            Ok(Response::new(TurnSessionStatistics {
                received_bytes: counts.received_bytes as u64,
                send_bytes: counts.send_bytes as u64,
                received_pkts: counts.received_pkts as u64,
                send_pkts: counts.send_pkts as u64,
                error_pkts: counts.error_pkts as u64,
            }))
        } else {
            Err(Status::not_found("Session not found"))
        }
    }

    async fn destroy_session(
        &self,
        request: Request<sdk::protos::Identifier>,
    ) -> Result<Response<()>, Status> {
        if self.service.get_session_manager().refresh(
            &Identifier::try_from(request.into_inner())
                .map_err(|e| Status::internal(e.to_string()))?,
            0,
        ) {
            Ok(Response::new(()))
        } else {
            Err(Status::failed_precondition("Session not found"))
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

                #[cfg(feature = "ssl")]
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

    pub async fn get_password(
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
                .get_password(Request::new(GetTurnPasswordRequest {
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

pub async fn start_server(config: Config, service: Service, statistics: Statistics) -> Result<()> {
    if let Some(api) = &config.api {
        let mut builder = Server::builder();

        builder = builder
            .timeout(Duration::from_secs(api.timeout as u64))
            .accept_http1(false);

        #[cfg(feature = "ssl")]
        if let Some(ssl) = &api.ssl {
            builder = builder.tls_config(ServerTlsConfig::new().identity(Identity::from_pem(
                ssl.certificate_chain.clone(),
                ssl.private_key.clone(),
            )))?;
        }

        log::info!("api server listening: listen={}", api.listen);

        builder
            .add_service(TurnServiceServer::new(RpcService {
                config: config.clone(),
                uptime: Instant::now(),
                statistics,
                service,
            }))
            .serve(api.listen)
            .await?;
    } else {
        std::future::pending().await
    }

    Ok(())
}
