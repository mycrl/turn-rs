pub mod proto {
    tonic::include_proto!("turn.server");
}

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use service::{
    Service,
    session::{Identifier, Session},
};

use tokio::sync::mpsc::{Sender, channel};
use tonic::{
    Request, Response, Status,
    transport::{Channel, Server},
};

#[cfg(feature = "ssl")]
use tonic::transport::{Certificate, ClientTlsConfig, Identity, ServerTlsConfig};

use self::proto::{
    SessionQueryParams, TurnAllocatedEvent, TurnChannelBindEvent,
    TurnCreatePermissionEvent, TurnDestroyEvent, TurnRefreshEvent, TurnServerInfo, TurnSession,
    TurnSessionStatistics,
    turn_hooks_service_client::TurnHooksServiceClient,
    turn_service_server::{TurnService, TurnServiceServer},
};

use crate::{config::Config, handler::Handler, statistics::Statistics};

pub trait IdString {
    type Error;

    fn to_string(&self) -> String;
    fn from_string(s: String) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

impl IdString for Identifier {
    type Error = anyhow::Error;

    fn to_string(&self) -> String {
        format!("{}/{}", self.source, self.interface)
    }

    fn from_string(s: String) -> Result<Self, Self::Error> {
        let (source, interface) = s.split_once('/').ok_or(anyhow!("Invalid identifier"))?;

        Ok(Self {
            source: source.parse()?,
            interface: interface.parse()?,
        })
    }
}

struct RpcService {
    config: Arc<Config>,
    service: Service<Handler>,
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
                .turn
                .get_externals()
                .iter()
                .map(|addr| addr.to_string())
                .collect(),
            port_capacity: self.config.runtime.port_range.size() as u32,
            port_allocated: self.service.get_session_manager_ref().allocated() as u32,
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
            .get_session_manager_ref()
            .get_session(
                &Identifier::from_string(request.into_inner().id)
                    .map_err(|_| Status::invalid_argument("Invalid identifier"))?,
            )
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
        if let Some(counts) = self.statistics.get(
            &Identifier::from_string(request.into_inner().id)
                .map_err(|_| Status::invalid_argument("Invalid identifier"))?,
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
        request: Request<SessionQueryParams>,
    ) -> Result<Response<()>, Status> {
        if self.service.get_session_manager_ref().refresh(
            &Identifier::from_string(request.into_inner().id)
                .map_err(|_| Status::invalid_argument("Invalid identifier"))?,
            0,
        ) {
            Ok(Response::new(()))
        } else {
            Err(Status::failed_precondition("Session not found"))
        }
    }
}

pub async fn start_server(
    config: Arc<Config>,
    service: Service<Handler>,
    statistics: Statistics,
) -> Result<()> {
    let mut builder = Server::builder();

    builder = builder.timeout(Duration::from_secs(5)).accept_http1(false);

    #[cfg(feature = "ssl")]
    if let Some(ssl) = &config.rpc.ssl {
        builder = builder.tls_config(ServerTlsConfig::new().identity(Identity::from_pem(
            ssl.certificate_chain.clone(),
            ssl.private_key.clone(),
        )))?;
    }

    builder
        .add_service(TurnServiceServer::new(RpcService {
            config: config.clone(),
            uptime: Instant::now(),
            statistics,
            service,
        }))
        .serve(config.rpc.listen)
        .await?;

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
}

pub struct RpcHooksService(Option<RpcHooksServiceInner>);

impl RpcHooksService {
    pub async fn new(config: &Config) -> Result<Self> {
        if let Some(hooks) = &config.rpc.hooks {
            let (event_channel, mut rx) = channel(hooks.max_channel_size);
            let mut client = {
                let mut builder = Channel::builder(hooks.endpoint.as_str().try_into()?);

                #[cfg(feature = "ssl")]
                if let Some(ssl) = &hooks.ssl {
                    builder = builder.tls_config(
                        ClientTlsConfig::new()
                            .ca_certificate(Certificate::from_pem(ssl.certificate_chain.clone()))
                            .domain_name(
                                url::Url::parse(&hooks.endpoint)?
                                    .domain()
                                    .ok_or_else(|| anyhow!("Invalid hooks server domain"))?,
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

            Ok(Self(Some(RpcHooksServiceInner {
                event_channel,
            })))
        } else {
            Ok(Self(None))
        }
    }

    pub fn send_event(&self, event: HooksEvent) {
        if let Some(inner) = &self.0 {
            if !inner.event_channel.is_closed() {
                if let Err(e) = inner.event_channel.try_send(event) {
                    log::error!("Failed to send event to hooks server: {}", e);
                }
            }
        }
    }
}
