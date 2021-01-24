mod service;
mod transport;

pub use service::Auth;

use service::*;
use anyhow::{
    Result,
    anyhow
};

use super::{
    state::State as States,
    config::Conf,
};

use std::{
    net::SocketAddr,
    sync::Arc,
};

use transport::Transport;
use tokio::net::TcpStream;

/// 控制器
pub struct Controls {
    inner: Arc<Transport>,
    state: Arc<States>,
}

impl Controls {
    /// 创建实例
    #[rustfmt::skip]
    pub async fn new(conf: Arc<Conf>, state: Arc<States>) -> Result<Arc<Self>> {
        let socket = TcpStream::connect(conf.controls).await?;
        Ok(Arc::new(Self {
            inner: Transport::new(socket),
            state
        }))
    }
    
    /// 获取认证信息
    #[rustfmt::skip]
    pub async fn auth(&self, u: &str, a: &SocketAddr) -> Result<Auth> {
        self.inner.call(Trigger::Auth as u8, &AuthRequest {
            username: u.to_string(),
            addr: *a
        }).await
    }
    
    pub async fn get(self: Arc<Self>) {
        self.inner.clone().bind(State::Get as u8, |req: GetRequest| async move {
            self.state.base.write().await
                .get(&req.addr)
                .map(Node::from)
                .ok_or_else(|| anyhow!("not found"))
        }).await
    }
}
