pub mod request;
pub mod response;

use response::Response;
use anyhow::{
    Result,
    Error
};
use std::{
    net::SocketAddr, 
    sync::Arc
};

use async_nats::{
    connect,
    Connection
};

use std::convert::{
    Into, 
    TryFrom
};

use super::{
    env::Environment,
    state::State
};

struct Topic {
    auth: String,
    close: String,
}

/// Bridge
///
/// The Bridge is the main component of turn. 
/// It handles services, calls actions, 
/// emits events and communicates with remote nodes. 
/// You must create a Bridge instance on every node.
pub struct Bridge {
    state: Option<Arc<State>>,
    nats: Connection,
    topic: Topic,
}

impl Bridge {
    /// connect nats server.
    pub async fn new(c: &Arc<Environment>) -> Result<Arc<Self>> {
        let bridge = Arc::new(Self { 
            nats: connect(c.nats.as_str()).await?,
            state: None,
            topic: Topic {
                auth: format!("auth.{}", c.realm),
                close: format!("{}.close", c.realm),
            }
        });

        let bridge_1 = bridge.clone();
        tokio::spawn(async move {
            bridge_1
                .handle_close()
                .await?;
            Ok::<(), Error>(())
        });
        
        Ok(bridge)
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
            .nats
            .request(
                &self.topic.auth, 
                Into::<Vec<u8>>::into(request::Auth { 
                    username: u.to_string(), 
                    addr: *a 
                })
            ).await?;
        Response
            ::<response::Auth>
            ::try_from(message.data.as_slice())?
            .into_result()
    }

    /// please allow me to make it work in an unsafe way, 
    /// I don’t want to continue to have a headache for 
    /// the initialization behavior.
    ///
    /// ```no_run
    /// let c = env::Environment::generate()?;
    /// let Bridge = Bridge::new(&c).await?;
    /// let source_addr = "127.0.0.1:8080".parse().unwrap();
    /// let res = Bridge.auth(&source_addr, "panda").await?;
    /// // res.password
    /// ```
    pub fn set_state(&self, state: Arc<State>) {
        unsafe {
            let const_ptr = self as *const Bridge;
            let mut_ptr = const_ptr as *mut Bridge;
            let mut_ref = &mut *mut_ptr;
            mut_ref.state = Some(state)
        }
    }

    /// please allow me to make it work in an unsafe way, 
    /// I don’t want to continue to have a headache for 
    /// the initialization behavior.
    ///
    /// ```no_run
    /// let c = env::Environment::generate()?;
    /// let Bridge = Bridge::new(&c).await?;
    /// let source_addr = "127.0.0.1:8080".parse().unwrap();
    /// let res = Bridge.auth(&source_addr, "panda").await?;
    /// // res.password
    /// ```
    async fn handle_close(self: Arc<Self>) -> Result<()> {
        let sub = self.nats
            .subscribe(&self.topic.close)
            .await?;

        let state = self
            .state
            .as_ref()
            .unwrap()
            .clone(); 
        while let Some(msg) = sub.next().await {
            let u = request
                ::Close
                ::try_from(msg.data)?
                .username;
            state
                .remove_from_user(&u)
                .await;
        }

        Ok(())
    }
}
