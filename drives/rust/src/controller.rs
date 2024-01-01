use std::{collections::HashMap, net::SocketAddr, str::FromStr};

use thiserror::Error;
use tokio::sync::Mutex;
use tonic::{transport::Channel, Request};

use super::proto::{controller_client::ControllerClient, AddrParams, QueryFilter};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    TCP = 0,
    UDP = 1,
}

#[derive(Debug, Clone)]
pub struct Interface {
    pub transport: Transport,
    pub bind: SocketAddr,
    pub external: SocketAddr,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub software: String,
    pub uptime: u64,
    pub capacity: u16,
    pub allocated: u16,
    pub realm: String,
    pub interfaces: Vec<Interface>,
}

#[derive(Debug, Clone, Copy)]
pub struct Report {
    pub received_bytes: u64,
    pub send_bytes: u64,
    pub received_pkts: u64,
    pub send_pkts: u64,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub username: String,
    pub password: String,
    pub lifetime: u64,
    pub timer: u64,
    pub channels: Vec<u16>,
    pub ports: Vec<u16>,
}

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error(transparent)]
    ConnectError(#[from] tonic::transport::Error),
    #[error(transparent)]
    RpcError(#[from] tonic::Status),
}

pub struct Controller(Mutex<ControllerClient<Channel>>);

impl Controller {
    pub async fn new(uri: &str) -> Result<Self, ControllerError> {
        Ok(Self(Mutex::new(
            ControllerClient::connect(uri.to_string()).await?,
        )))
    }

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
    pub async fn get_status(&self) -> Result<Stats, ControllerError> {
        let stats = self
            .0
            .lock()
            .await
            .get_stats(Request::new(()))
            .await?
            .into_inner();

        Ok(Stats {
            software: stats.software,
            uptime: stats.uptime,
            capacity: stats.port_capacity as u16,
            allocated: stats.port_allocated as u16,
            realm: stats.realm,
            interfaces: stats
                .interfaces
                .into_iter()
                .map(|item| {
                    Ok::<_, <SocketAddr as FromStr>::Err>(Interface {
                        bind: item.bind.parse()?,
                        external: item.external.parse()?,
                        transport: if item.transport == 0 {
                            Transport::TCP
                        } else {
                            Transport::UDP
                        },
                    })
                })
                .flatten()
                .collect(),
        })
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
    pub async fn get_users(
        &self,
        skip: u32,
        limit: u32,
    ) -> Result<HashMap<String, HashMap<SocketAddr, Report>>, ControllerError> {
        let users = self
            .0
            .lock()
            .await
            .get_users(Request::new(QueryFilter {
                skip: Some(skip),
                limit: Some(limit),
            }))
            .await?
            .into_inner()
            .users;

        Ok(users
            .into_iter()
            .map(|user| {
                (
                    user.name,
                    user.reports
                        .into_iter()
                        .map(|item| {
                            Ok::<_, <SocketAddr as FromStr>::Err>((
                                item.addr.parse()?,
                                Report {
                                    received_bytes: item.received_bytes,
                                    send_bytes: item.send_bytes,
                                    received_pkts: item.received_pkts,
                                    send_pkts: item.send_pkts,
                                },
                            ))
                        })
                        .flatten()
                        .collect(),
                )
            })
            .collect())
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
    pub async fn get_session(&self, addr: &SocketAddr) -> Result<Option<Session>, ControllerError> {
        Ok(self
            .0
            .lock()
            .await
            .get_session(Request::new(AddrParams {
                addr: addr.to_string(),
            }))
            .await?
            .into_inner()
            .session
            .map(|item| Session {
                timer: item.timer,
                username: item.username,
                password: item.password,
                lifetime: item.lifetime,
                channels: item.channels.into_iter().map(|v| v as u16).collect(),
                ports: item.ports.into_iter().map(|v| v as u16).collect(),
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
    pub async fn remove_session(&self, addr: &SocketAddr) -> Result<(), ControllerError> {
        self.0
            .lock()
            .await
            .remove_session(Request::new(AddrParams {
                addr: addr.to_string(),
            }))
            .await?;
        Ok(())
    }
}
