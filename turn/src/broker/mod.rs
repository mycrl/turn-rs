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

/// 
pub struct Broker {
    nats: Connection
}

impl Broker {
    pub async fn new(a: &str) -> Result<Arc<Self>> {
        Ok(Arc::new(Self { nats: connect(a).await? }))
    }
    
    #[rustfmt::skip]
    pub async fn auth(&self, a: &SocketAddr, u: &str) -> Result<response::Auth> {
        let req = request::Auth { username: u.to_string(), addr: a.clone() };
        let message = self.nats.request("auth", Into::<Vec<u8>>::into(req)).await?;
        Response::<response::Auth>::try_from(message.data.as_slice())?.into_result()
    }
}
