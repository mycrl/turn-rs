pub mod request;
pub mod response;

use async_nats::Connection;
use response::Response;
use anyhow::{
    Result,
    Error
};

use super::{
    env::Environment,
    state::State,
};

use std::{
    net::SocketAddr, 
    sync::Arc
};

use std::convert::{
    Into, 
    TryFrom
};

/// Bridge Client
///
/// The Bridge is the main component of turn. 
/// It handles services, calls actions, 
/// emits events and communicates with remote nodes. 
/// You must create a Bridge instance on every node.
pub struct Publish {
    conn: Connection,
    env: Arc<Environment>,
}

impl Publish {
    /// connect nats publish.
    pub fn new(env: &Arc<Environment>, conn: Connection) -> Arc<Self> {
        Arc::new(Self { 
            env: env.clone(),
            conn
        })
    }
    
    /// provide the user name and source address, 
    /// request the control service to give the 
    /// key of the current user.
    ///
    /// ```no_run
    /// let c = env::Environment::generate()?;
    /// let Bridge = Bridge::new(&c).await?;
    /// let source_addr = "127.0.0.1:8080".parse().unwrap();
    /// let res = Bridge.auth(&source_addr, "panda").await?;
    /// // res.password
    /// ```
    #[rustfmt::skip]
    pub async fn auth(&self, a: &SocketAddr, u: &str) -> Result<response::Auth> {
        let message = self
            .conn
            .request(
                "turn.auth", 
                Into::<Vec<u8>>::into(request::Auth { 
                    realm: self.env.realm.to_string(),
                    username: u.to_string(), 
                    addr: *a 
                })
            ).await?;
        Response
            ::<response::Auth>
            ::try_from(message.data.as_slice())?
            .into_result()
    }
}

/// create nats subscribe.
///
/// ```no_run
/// let c = env::Environment::generate()?;
/// let Bridge = Bridge::new(&c).await?;
/// let source_addr = "127.0.0.1:8080".parse().unwrap();
/// let res = Bridge.auth(&source_addr, "panda").await?;
/// // res.password
/// ```
#[rustfmt::skip]
pub async fn create_subscribe(env: &Arc<Environment>, conn: Connection, state: Arc<State>) -> Result<()> {
    let sub = conn.subscribe(&format!("turn.{}.close", env.realm)).await?;
    tokio::spawn(async move {
        while let Some(message) = sub.next().await {
            if let Ok(close) = request::Close::try_from(message.data.as_slice()) {
                state.remove_from_user(&close.username).await;
                let res = Response::<()>::from(None, None);
                message.respond(Into::<Vec<u8>>::into(res)).await?;   
            }
        }

        Ok::<(), Error>(())
    });
    
    Ok(())
}
