mod service;
mod transport;

pub use service::*;
use transport::Transport;
use tokio::net::TcpStream;
use anyhow::{
    Result,
    anyhow
};

use super::{
    state::State,
    config::Conf,
};

use std::{
    net::SocketAddr,
    sync::Arc,
};

/// 控制器
pub struct Controls {
    inner: Arc<Transport>,
    state: Arc<State>,
}

impl Controls {
    /// 创建实例
    ///
    /// # Example
    ///
    /// ```no_run
    /// use super::{
    ///     config::Conf,
    ///     state::State,
    ///     Controls
    /// };
    /// 
    /// let config = config::new();
    /// let state = State::new();
    /// let controls = Controls::new(config, state).await?;
    /// ```
    #[rustfmt::skip]
    pub async fn new(conf: Arc<Conf>, state: Arc<State>) -> Result<Arc<Self>> {
        let socket = TcpStream::connect(conf.controls).await?;
        let myself = Arc::new(Self {
            inner: Transport::new(socket).run(),
            state
        });

        myself.clone().run_get_service().await;
        myself.clone().run_remove_service().await;
        Ok(myself)
    }

    /// 获取认证信息
    ///
    /// # Example
    ///
    /// ```no_run
    /// use super::{
    ///     config::Conf,
    ///     state::State,
    ///     Controls
    /// };
    /// 
    /// let config = config::new();
    /// let state = State::new();
    /// let controls = Controls::new(config, state).await?;
    /// let auth = controls.auth("panda", "127.0.0.1:8080".parse()?).await?;
    /// ```
    #[rustfmt::skip]
    pub async fn auth(&self, u: &str, a: &SocketAddr) -> Result<Auth> {
        self.inner.call(Trigger::Auth as u8, &AuthRequest {
            username: u.to_string(),
            addr: *a
        }).await
    }

    /// 启动节点信息获取服务
    ///
    /// 控制中心通过指定客户端地址来获取节点信息
    async fn run_get_service(self: Arc<Self>) {
        self.inner.clone().bind(Service::Get as u8, move |req: Request| {
            let state = self.state.clone();
            async move {
                state.base.read().await
                    .get(&req.addr)
                    .map(Node::from)
                    .ok_or_else(|| anyhow!("not found"))
            }
        }).await
    }

    /// 启动节点删除服务
    ///
    /// 控制中心通过指定客户端地址来删除并停止节点
    async fn run_remove_service(self: Arc<Self>) {
        self.inner.clone().bind(Service::Remove as u8, move |req: Request| {
            let state = self.state.clone();
            async move {
                Ok(state.remove(&Arc::new(req.addr)).await)
            }
        }).await
    }
}
