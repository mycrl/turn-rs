use std::time::Instant;

use crate::{
    Service,
    config::Config,
    service::session::{Identifier, Session},
    statistics::Statistics,
};

use anyhow::Result;
use tonic::{Request, Response, Status};

use sdk::protos::{
    BindAddress, TurnServerInfo, TurnSession, TurnSessionStatistics,
    turn_service_server::TurnService,
};

pub struct RpcServer {
    pub config: Config,
    pub service: Service,
    pub statistics: Statistics,
    pub uptime: Instant,
}

#[tonic::async_trait]
impl TurnService for RpcServer {
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
