pub mod request;
pub mod response;

use async_trait::async_trait;
use response::Response;
use anyhow::Result;
use std::{
    net::SocketAddr,
    sync::Arc, 
};

use super::{
    env::Environment,
    router::Router,
};

use rpc::{
    Caller,
    Servicer,
    RpcCaller
};

/// close action service.
pub struct Close {
    router: Arc<Router>,
    env: Arc<Environment>,
}

impl Close {
    pub fn new(router: &Arc<Router>, env: &Arc<Environment>) -> Self {
        Self {
            router: router.clone(),
            env: env.clone()
        }
    }
}

#[async_trait]
impl Servicer<request::Close, Response<()>> for Close {
    fn topic(&self) -> String {
        format!("turn.{}.close", self.env.realm)
    }
    
    /// uncheck input serialization.
    ///
    /// # Example
    ///
    /// ```no_run
    /// Into::<Auth>::into(Auth {
    ///     addr: "127.0.0.1:8080".parse().unwrap(),
    ///     username: "panda".to_string()
    /// })
    /// ```
    async fn handler(&self, message: request::Close) -> Response<()> {
        let res = self.router
            .remove_from_user(&message.username)
            .await;
        Response::<()>::from(match res {
            None => Some("user is not found!".to_string()),
            Some(_) => None,
        }, None)
    }
}

/// auth request service.
pub struct Auth {
    env: Arc<Environment>,
}

/// auth service caller type.
pub type AuthCaller = RpcCaller<
    (SocketAddr, String), 
    response::Auth
>;

impl Auth {
    pub fn new(env: &Arc<Environment>) -> Self {
        Self { env: env.clone() }
    }
}

impl Caller<(SocketAddr, String), response::Auth> for Auth {
    fn topic(&self) -> String {
        "turn.auth".to_string()
    }
    
    /// uncheck input serialization.
    ///
    /// # Example
    ///
    /// ```no_run
    /// Into::<Auth>::into(Auth {
    ///     addr: "127.0.0.1:8080".parse().unwrap(),
    ///     username: "panda".to_string()
    /// })
    /// ```
    fn serializer(&self, (addr, username): (SocketAddr, String)) -> Vec<u8> {
        Into::<Vec<u8>>::into(request::Auth { 
            realm: self.env.realm.to_string(),
            username, 
            addr
        })
    }

    /// uncheck input serialization.
    ///
    /// # Example
    ///
    /// ```no_run
    /// Into::<Auth>::into(Auth {
    ///     addr: "127.0.0.1:8080".parse().unwrap(),
    ///     username: "panda".to_string()
    /// })
    /// ```
    fn deserializer(&self, data: &[u8]) -> Result<response::Auth> {
        Response
            ::<response::Auth>
            ::try_from(data)?
            .into_result()
    }
}

/// state action service.
pub struct State {
    router: Arc<Router>,
    env: Arc<Environment>,
}

impl State {
    pub fn new(router: &Arc<Router>, env: &Arc<Environment>) -> Self {
        Self {
            router: router.clone(),
            env: env.clone()
        }
    }
}

#[async_trait]
impl Servicer<(), Response<response::State>> for State {
    fn topic(&self) -> String {
        format!("turn.{}.state", self.env.realm)
    }
    
    /// uncheck input serialization.
    ///
    /// # Example
    ///
    /// ```no_run
    /// Into::<Auth>::into(Auth {
    ///     addr: "127.0.0.1:8080".parse().unwrap(),
    ///     username: "panda".to_string()
    /// })
    /// ```
    async fn handler(&self, _: ()) -> Response<response::State> {
        Response::<response::State>::from(None, Some(response::State {
            capacity: self.router.capacity().await,
            users: self.router.get_users().await,
            len: self.router.len().await,
        }))
    }
}

/// state action service.
pub struct Node {
    router: Arc<Router>,
    env: Arc<Environment>,
}

impl Node {
    pub fn new(router: &Arc<Router>, env: &Arc<Environment>) -> Self {
        Self {
            router: router.clone(),
            env: env.clone()
        }
    }
}

#[async_trait]
impl Servicer<request::Node, Response<response::Node>> for Node {
    fn topic(&self) -> String {
        format!("turn.{}.node", self.env.realm)
    }
    
    /// uncheck input serialization.
    ///
    /// # Example
    ///
    /// ```no_run
    /// Into::<Auth>::into(Auth {
    ///     addr: "127.0.0.1:8080".parse().unwrap(),
    ///     username: "panda".to_string()
    /// })
    /// ```
    async fn handler(&self, req: request::Node) -> Response<response::Node> {
        let res = self.router
            .get_node(&req.username)
            .await
            .map(|n| response::Node::from(n));
        Response::<response::Node>::from(match res {
            None => Some("user is not found!".to_string()),
            Some(_) => None,
        }, res)
    }
}
