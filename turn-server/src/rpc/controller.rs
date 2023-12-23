use super::{
    proto::{
        controller_server::Controller, AddrParams, GetReportReply, GetSessionReply, GetUsersReply,
        Interface, QueryFilter, Report, Session, Stats, User,
    },
    SOFTWARE,
};

use crate::{config::Config, monitor::Monitor};
use std::{net::SocketAddr, sync::Arc, time::Instant};

use tonic::{Request, Response, Status};
use turn_rs::Service;

pub struct ControllerService {
    config: Arc<Config>,
    service: Service,
    monitor: Monitor,
    timer: Instant,
}

impl ControllerService {
    pub fn new(config: Arc<Config>, monitor: Monitor, service: Service) -> Self {
        Self {
            timer: Instant::now(),
            monitor,
            service,
            config,
        }
    }
}

#[tonic::async_trait]
impl Controller for ControllerService {
    /// Gets status information about the current turn server, including
    /// version, startup duration, domain, assigned ports, total port capacity,
    /// and list of bound interfaces.
    ///
    /// WARNING: It is important to note that the returned results of this
    /// interface will contain private information, so please protect the data
    /// primarily from disclosure to external networks.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message Stats {
    ///     string software = 1;
    ///     uint64 uptime = 3;
    ///     uint32 port_capacity = 4;
    ///     uint32 port_allocated = 5;
    ///     string realm = 6;
    ///     repeated Interface interfaces = 2;
    /// }
    ///
    /// rpc GetStats (google.protobuf.Empty) returns (Stats)
    /// ```
    async fn get_stats(&self, _: Request<()>) -> Result<Response<Stats>, Status> {
        let router = self.service.get_router();
        Ok(Response::new(Stats {
            software: SOFTWARE.to_string(),
            uptime: self.timer.elapsed().as_secs(),
            realm: self.config.turn.realm.clone(),
            port_allocated: router.len() as u32,
            port_capacity: router.capacity() as u32,
            interfaces: self
                .config
                .turn
                .interfaces
                .iter()
                .map(|item| Interface {
                    transport: item.transport as i32,
                    bind: item.bind.to_string(),
                    external: item.external.to_string(),
                })
                .collect(),
        }))
    }

    /// Get the traffic statistics of each session of the turn server, such as
    /// how much data received, how much data sent, how many stun packets
    /// received, how many stun packets sent, please note that this data is
    /// cumulative and the maximum is u64::MAX, if exceeded it will be
    /// automatically cleared and recounted.
    ///
    /// WARNING: It is important to note that the returned results of this
    /// interface will contain private information, so please protect the data
    /// primarily from disclosure to external networks.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message QueryFilter {
    ///     optional uint32 skip = 1;
    ///     optional uint32 limit = 2;
    /// }
    ///
    /// message Report {
    ///     string addr = 1;
    ///     uint64 received_bytes = 2;
    ///     uint64 send_bytes = 3;
    ///     uint64 received_pkts = 4;
    ///     uint64 send_pkts = 5;
    /// }
    ///
    /// message GetReportReply {
    ///     repeated Report reports = 1;
    /// }
    ///
    /// rpc GetReport (QueryFilter) returns (GetReportReply);
    /// ```
    async fn get_report(
        &self,
        request: Request<QueryFilter>,
    ) -> Result<Response<GetReportReply>, Status> {
        Ok(Response::new(GetReportReply {
            reports: self
                .monitor
                .get_nodes(
                    request.get_ref().skip.unwrap_or(0) as usize,
                    request.get_ref().limit.unwrap_or(20) as usize,
                )
                .into_iter()
                .map(|(addr, item)| Report {
                    addr: addr.to_string(),
                    received_bytes: item.received_bytes as u64,
                    received_pkts: item.received_pkts as u64,
                    send_bytes: item.send_bytes as u64,
                    send_pkts: item.send_pkts as u64,
                })
                .collect(),
        }))
    }

    /// Get the list of connected users on the turn server and all the network
    /// addresses used by the current user. Note that a user can use more than
    /// one network address to communicate with the turn server at the same
    /// time, so the network addresses are a list.
    ///
    /// WARNING: It is important to note that the returned results of this
    /// interface will contain private information, so please protect the data
    /// primarily from disclosure to external networks.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message QueryFilter {
    ///     optional uint32 skip = 1;
    ///     optional uint32 limit = 2;
    /// }
    ///
    /// message User {
    ///     string name = 1;
    ///     repeated string addrs = 2;
    /// }
    ///
    /// message GetUsersReply {
    ///     repeated User users = 1;
    /// }
    ///
    /// rpc GetUsers (QueryFilter) returns (GetUsersReply);
    /// ```
    async fn get_users(
        &self,
        request: Request<QueryFilter>,
    ) -> Result<Response<GetUsersReply>, Status> {
        Ok(Response::new(GetUsersReply {
            users: self
                .service
                .get_router()
                .get_users(
                    request.get_ref().skip.unwrap_or(0) as usize,
                    request.get_ref().limit.unwrap_or(20) as usize,
                )
                .into_iter()
                .map(|(name, addrs)| User {
                    name: name.to_string(),
                    addrs: addrs.into_iter().map(|addr| addr.to_string()).collect(),
                })
                .collect(),
        }))
    }

    /// Get session information for a particular user, including a list of
    /// assigned channel numbers, a list of assigned port numbers, time alive,
    /// time consumed, username password, and so on.
    ///
    /// WARNING: It is important to note that the returned results of this
    /// interface will contain private information, so please protect the data
    /// primarily from disclosure to external networks.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message AddrParams {
    ///     string addr = 1;
    /// }
    ///
    /// message Session {
    ///     string username = 1;
    ///     string password = 2;
    ///     uint64 lifetime = 3;
    ///     uint64 timer = 4;
    ///     repeated uint32 channels = 5;
    ///     repeated uint32 ports = 6;
    /// }
    ///
    /// message GetSessionReply {
    ///     optional Session node = 1;
    /// }
    ///
    /// rpc GetSession (AddrParams) returns (GetSessionReply);
    /// ```
    async fn get_session(
        &self,
        request: Request<AddrParams>,
    ) -> Result<Response<GetSessionReply>, Status> {
        Ok(Response::new(GetSessionReply {
            session: self
                .service
                .get_router()
                .get_node(
                    &request
                        .get_ref()
                        .addr
                        .parse::<SocketAddr>()
                        .map_err(|e| Status::from_error(Box::new(e)))?,
                )
                .map(|item| Session {
                    channels: item.channels.into_iter().map(|item| item as u32).collect(),
                    ports: item.ports.into_iter().map(|item| item as u32).collect(),
                    timer: item.timer.elapsed().as_secs(),
                    username: item.username,
                    password: item.password,
                    lifetime: item.lifetime,
                }),
        }))
    }

    /// Delete a user's session, it should be noted that deleting the session
    /// will cause the current user to disconnect directly, and the other end
    /// will also disconnect, but both sides can still apply for a session
    /// again, deletion does not mean blackout.
    ///
    /// # Proto
    ///
    /// ```proto
    /// message AddrParams {
    ///     string addr = 1;
    /// }
    ///
    /// rpc RemoveSession (AddrParams) returns (google.protobuf.Empty);
    /// ```
    async fn remove_session(&self, request: Request<AddrParams>) -> Result<Response<()>, Status> {
        self.service.get_router().remove(
            &request
                .get_ref()
                .addr
                .parse::<SocketAddr>()
                .map_err(|e| Status::from_error(Box::new(e)))?,
        );

        Ok(Response::new(()))
    }
}
