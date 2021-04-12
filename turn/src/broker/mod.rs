pub mod request;
pub mod response;

use response::Response;
use anyhow::Result;
use std::{
    sync::Arc,
    net::SocketAddr,
};

use async_nats::{
    connect,
    Connection
};

use std::convert::{
    Into, 
    TryFrom
};

/// Broker
///
/// The Broker is the main component of turn. 
/// It handles services, calls actions, 
/// emits events and communicates with remote nodes. 
/// You must create a Broker instance on every node.
pub struct Broker {
    nats: Connection
}

impl Broker {
    /// connect nats server.
    ///
    /// # Example
    ///
    /// ```no_run
    /// // Broker::new("127.0.0.1:4222").await?
    /// ```
    pub async fn new(a: &str) -> Result<Arc<Self>> {
        Ok(Arc::new(Self { nats: connect(a).await? }))
    }
    
    /// provide the user name and source address, 
    /// request the control service to give the 
    /// key of the current user.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let source_addr = "127.0.0.1:8080".parse().unwrap();
    /// let broker = Broker::new("127.0.0.1:4222").await?;
    /// let res = broker.auth(&source_addr, "panda").await?;
    /// // res.password
    /// ```
    #[rustfmt::skip]
    pub async fn auth(&self, a: &SocketAddr, u: &str) -> Result<response::Auth> {
        let req = request::Auth { username: u.to_string(), addr: a.clone() };
        let message = self.nats.request("auth", Into::<Vec<u8>>::into(req)).await?;
        Response::<response::Auth>::try_from(message.data.as_slice())?.into_result()
    }
}
