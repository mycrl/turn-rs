pub mod request;
pub mod response;

use async_trait::async_trait;
use response::Response;
use anyhow::Result;
use std::{
    net::SocketAddr,
    sync::Arc,
};

use turn::Router;
use super::args::Args;

use trpc::{
    Caller,
    Servicer,
    RpcCaller,
};

/// close action service.
pub struct Close {
    router: Arc<Router>,
    args: Arc<Args>,
}

impl Close {
    pub fn new(router: &Arc<Router>, args: &Arc<Args>) -> Self {
        Self {
            router: router.clone(),
            args: args.clone(),
        }
    }
}

#[async_trait]
impl Servicer<request::Close, Response<()>> for Close {
    fn topic(&self) -> String {
        format!("turn.{}.close", self.args.realm)
    }

    /// uncheck input serialization.
    ///
    /// # Example
    ///
    /// ```no_run
    /// Into::<Auth>::into(Auth {
    ///     addr: "127.0.0.1:8080".parse().unwrap(),
    ///     username: "panda".to_string(),
    /// })
    /// ```
    async fn handler(&self, message: request::Close) -> Response<()> {
        self.router.remove_from_user(&message.username).await;
        Response::<()>::from(None, None)
    }
}

/// auth request service.
pub struct Auth {
    args: Arc<Args>,
}

/// auth service caller type.
pub type AuthCaller = RpcCaller<(SocketAddr, String), String>;

impl Auth {
    pub fn new(args: &Arc<Args>) -> Self {
        Self {
            args: args.clone(),
        }
    }
}

impl Caller<(SocketAddr, String), String> for Auth {
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
    ///     username: "panda".to_string(),
    /// })
    /// ```
    fn serializer(&self, (addr, id): (SocketAddr, String)) -> Vec<u8> {
        Into::<Vec<u8>>::into(request::Auth {
            realm: self.args.realm.to_string(),
            id,
            addr,
        })
    }

    /// uncheck input serialization.
    ///
    /// # Example
    ///
    /// ```no_run
    /// Into::<Auth>::into(Auth {
    ///     addr: "127.0.0.1:8080".parse().unwrap(),
    ///     username: "panda".to_string(),
    /// })
    /// ```
    fn deserializer(&self, data: &[u8]) -> Result<String> {
        Response::<response::Auth>::try_from(data)?
            .into_result()
            .map(|s| s.secret)
    }
}

/// state action service.
pub struct State {
    router: Arc<Router>,
    args: Arc<Args>,
}

impl State {
    pub fn new(router: &Arc<Router>, args: &Arc<Args>) -> Self {
        Self {
            router: router.clone(),
            args: args.clone(),
        }
    }
}

#[async_trait]
impl Servicer<(), Response<response::State>> for State {
    fn topic(&self) -> String {
        format!("turn.{}.state", self.args.realm)
    }

    /// uncheck input serialization.
    ///
    /// # Example
    ///
    /// ```no_run
    /// Into::<Auth>::into(Auth {
    ///     addr: "127.0.0.1:8080".parse().unwrap(),
    ///     username: "panda".to_string(),
    /// })
    /// ```
    async fn handler(&self, _: ()) -> Response<response::State> {
        Response::<response::State>::from(
            None,
            Some(response::State {
                capacity: self.router.capacity().await,
                users: self.router.get_users().await,
                len: self.router.len().await,
            }),
        )
    }
}

/// state action service.
pub struct Node {
    router: Arc<Router>,
    args: Arc<Args>,
}

impl Node {
    pub fn new(router: &Arc<Router>, args: &Arc<Args>) -> Self {
        Self {
            router: router.clone(),
            args: args.clone(),
        }
    }
}

#[async_trait]
impl Servicer<request::Node, Response<Vec<response::Node>>> for Node {
    fn topic(&self) -> String {
        format!("turn.{}.node", self.args.realm)
    }

    /// uncheck input serialization.
    ///
    /// # Example
    ///
    /// ```no_run
    /// Into::<Auth>::into(Auth {
    ///     addr: "127.0.0.1:8080".parse().unwrap(),
    ///     username: "panda".to_string(),
    /// })
    /// ```
    async fn handler(
        &self,
        req: request::Node,
    ) -> Response<Vec<response::Node>> {
        Response::<Vec<response::Node>>::from(
            None,
            Some(
                self.router
                    .get_nodes(&req.username)
                    .await
                    .into_iter()
                    .map(|n| response::Node::from(n))
                    .collect(),
            ),
        )
    }
}
