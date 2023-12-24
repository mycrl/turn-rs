use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;
use tonic::{transport::Channel, Request};

use self::proto::{
    controller_client::ControllerClient, AddrParams, GetSessionReply, GetUsersReply, QueryFilter,
    Stats,
};

pub mod proto {
    tonic::include_proto!("turn");
}

pub struct Rpc(Mutex<ControllerClient<Channel>>);

impl Rpc {
    pub async fn new(uri: &str) -> Result<Arc<Self>> {
        Ok(Arc::new(Self(Mutex::new(
            ControllerClient::connect(uri.to_string()).await?,
        ))))
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
    pub async fn get_status(&self) -> Result<Stats> {
        Ok(self
            .0
            .lock()
            .await
            .get_stats(Request::new(()))
            .await?
            .into_inner())
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
    pub async fn get_users(&self, skip: u32, limit: u32) -> Result<GetUsersReply> {
        Ok(self
            .0
            .lock()
            .await
            .get_users(Request::new(QueryFilter {
                skip: Some(skip),
                limit: Some(limit),
            }))
            .await?
            .into_inner())
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
    pub async fn get_session(&self, addr: String) -> Result<GetSessionReply> {
        Ok(self
            .0
            .lock()
            .await
            .get_session(Request::new(AddrParams { addr }))
            .await?
            .into_inner())
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
    pub async fn remove_session(&self, addr: String) -> Result<()> {
        self.0
            .lock()
            .await
            .remove_session(Request::new(AddrParams { addr }))
            .await?;
        Ok(())
    }
}
