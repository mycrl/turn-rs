use std::time::{Duration, Instant};

use anyhow::Result;
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

use protos::{
    GetTurnPasswordRequest, PasswordAlgorithm as ProtoPasswordAlgorithm, SessionQueryParams,
    TurnAllocatedEvent, TurnChannelBindEvent, TurnCreatePermissionEvent, TurnDestroyEvent,
    TurnRefreshEvent, TurnServerInfo, TurnSession, TurnSessionStatistics,
    turn_hooks_service_client::TurnHooksServiceClient,
    turn_service_server::{TurnService, TurnServiceServer},
};

use crate::{
    Service,
    codec::{crypto::Password, message::attributes::PasswordAlgorithm},
    config::Config,
    service::session::{Identifier, Session},
    statistics::Statistics,
};

impl From<PasswordAlgorithm> for ProtoPasswordAlgorithm {
    fn from(val: PasswordAlgorithm) -> Self {
        match val {
            PasswordAlgorithm::Md5 => Self::Md5,
            PasswordAlgorithm::Sha256 => Self::Sha256,
        }
    }
}

pub trait IdString {
    type Error;

    fn to_string(&self) -> String;
    fn from_string(s: String) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

impl IdString for Identifier {
    type Error = Status;

    fn to_string(&self) -> String {
        format!("{}/{}", self.source(), self.interface())
    }

    fn from_string(s: String) -> Result<Self, Self::Error> {
        let (source, interface) = s
            .split_once('/')
            .ok_or(Status::invalid_argument("Invalid identifier"))?;

        Ok(Self::new(
            source
                .parse()
                .map_err(|_| Status::invalid_argument("Invalid source address"))?,
            interface
                .parse()
                .map_err(|_| Status::invalid_argument("Invalid interface address"))?,
        ))
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
                .get_external_addresses()
                .iter()
                .map(|addr| addr.to_string())
                .collect(),
            port_capacity: self.config.server.port_range.size() as u32,
            port_allocated: self.service.get_session_manager().allocated() as u32,
        }))
    }

    async fn get_session(
        &self,
        request: Request<SessionQueryParams>,
    ) -> Result<Response<TurnSession>, Status> {
        if let Some(Session::Authenticated {
            username,
            allocate_port,
            allocate_channels,
            permissions,
            expires,
            ..
        }) = self
            .service
            .get_session_manager()
            .get_session(&Identifier::from_string(request.into_inner().id)?)
            .get_ref()
        {
            Ok(Response::new(TurnSession {
                username: username.to_string(),
                permissions: permissions.iter().map(|p| *p as i32).collect(),
                channels: allocate_channels.iter().map(|p| *p as i32).collect(),
                port: allocate_port.map(|p| p as i32),
                expires: *expires as i64,
            }))
        } else {
            Err(Status::not_found("Session not found"))
        }
    }

    async fn get_session_statistics(
        &self,
        request: Request<SessionQueryParams>,
    ) -> Result<Response<TurnSessionStatistics>, Status> {
        if let Some(counts) = self
            .statistics
            .get(&Identifier::from_string(request.into_inner().id)?)
        {
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
        request: Request<SessionQueryParams>,
    ) -> Result<Response<()>, Status> {
        if self
            .service
            .get_session_manager()
            .refresh(&Identifier::from_string(request.into_inner().id)?, 0)
        {
            Ok(Response::new(()))
        } else {
            Err(Status::failed_precondition("Session not found"))
        }
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
                                url::Url::parse(&hooks.endpoint)?
                                    .domain()
                                    .ok_or_else(|| anyhow::anyhow!("Invalid hooks server domain"))?,
                            ),
                    )?;
                }

                TurnHooksServiceClient::new(
                    builder
                        .connect_timeout(Duration::from_secs(5))
                        .timeout(Duration::from_secs(1))
                        .connect()
                        .await?,
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
        username: &str,
        algorithm: PasswordAlgorithm,
    ) -> Option<Password> {
        if let Some(inner) = &self.0 {
            let algorithm: ProtoPasswordAlgorithm = algorithm.into();
            let password = inner
                .client
                .lock()
                .await
                .get_password(Request::new(GetTurnPasswordRequest {
                    username: username.to_string(),
                    algorithm: algorithm as i32,
                }))
                .await
                .ok()?
                .into_inner()
                .password;

            return Some(match algorithm {
                ProtoPasswordAlgorithm::Md5 => Password::Md5(password.try_into().ok()?),
                ProtoPasswordAlgorithm::Sha256 => Password::Sha256(password.try_into().ok()?),
                ProtoPasswordAlgorithm::Unspecified => unreachable!(),
            });
        }

        None
    }
}
