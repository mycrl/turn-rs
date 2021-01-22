mod service;

pub use service::Auth;

use service::*;
use anyhow::Result;
use tokio::sync::RwLock;
use super::{
    state::State as States,
    config::Conf,
};

use std::{
    net::SocketAddr,
    sync::Arc,
};

use tarpc::{
    serde_transport::tcp as Socket,
    tokio_serde::formats::Json,
    client::Config,
};

use tarpc::context::{
    Context,
    current
};

/// 控制器
///
/// 外部控制API抽象
///
/// * `inner` 内部RPC实例
/// * `state` 状态管理
#[derive(Clone)]
pub struct Controls {
    inner: Arc<RwLock<TriggerClient>>,
    state: Arc<States>,
}

impl Controls {
    /// 创建实例
    ///
    /// # Example
    ///
    /// ```
    /// use super::*;
    ///
    /// let conf = config::new().unwrap();
    /// let controls = Controls::new(conf);
    /// ```
    #[rustfmt::skip]
    pub async fn new(conf: Arc<Conf>, state: Arc<States>) -> Result<Arc<Self>> {
        let socket = Socket::connect(
            conf.controls, 
            Json::default
        ).await?;
        
        let inner = TriggerClient::new(
            Config::default(), 
            socket
        ).spawn()?;
        
        Ok(Arc::new(Self {
            inner: Arc::new(RwLock::new(inner)),
            state
        }))
    }
    
    /// 获取认证信息
    ///
    /// # Example
    ///
    /// ```
    /// use super::*;
    ///
    /// let conf = config::new().unwrap();
    /// let controls = Controls::new(conf);
    /// let addr = "127.0.0.1:8080".parse().unwrap();
    /// let auth = controls.auth("panda", &addr);
    /// ```
    #[rustfmt::skip]
    pub async fn auth(&self, u: &str, a: &SocketAddr) -> Result<Auth> {
        let req = AuthRequest {
            username: u.to_string(),
            addr: a.clone(),
        };
        
        let res = self.inner
            .write()
            .await
            .auth(current(), req)
            .await?;
        Ok(res)
    }
}

#[tarpc::server]
impl State for Controls {
    /// 获取节点信息
    ///
    /// 控制中心获取对应地址节点信息
    async fn get(self, _: Context, addr: SocketAddr) -> Option<Node> {
        self.state.base.read().await.get(&addr).map(Node::from)
    }
    
    /// 删除节点
    ///
    /// 控制中心删除节点信息
    /// 这会导致清空节点所有信息并注销节点的所有通道
    async fn remove(self, _: Context, addr: SocketAddr) {
        self.state.remove(&Arc::new(addr)).await;
    }
}