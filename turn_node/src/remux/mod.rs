mod allocate;
mod binding;
mod channel_bind;
mod channel_data;
mod create_permission;
mod indication;
mod refresh;

use anyhow::Result;
use bytes::BytesMut;
use super::{
    controls::Controls,
    config::Conf,
    state::State,
};

use std::{
    convert::TryFrom, 
    net::SocketAddr, 
    sync::Arc
};

use crate::payload::{
    Kind, 
    Payload,
    Message,
};

#[rustfmt::skip]
static SOFTWARE: &str = concat!(
    "Mysticeti ",
    env!("CARGO_PKG_VERSION")
);

#[rustfmt::skip]
pub(crate) type Response<'a> = Option<(
    &'a [u8],
    Arc<SocketAddr>
)>;

/// 上下文
pub struct Context {
    pub conf: Arc<Conf>,
    pub state: Arc<State>,
    
    /// 会话端地址
    pub addr: Arc<SocketAddr>,
    pub controls: Arc<Controls>,
    
    /// 本地绑定地址
    pub local: Arc<SocketAddr>,
}

/// 解复用
pub struct Remux {
    controls: Arc<Controls>,
    local: Arc<SocketAddr>,
    conf: Arc<Conf>,
    state: Arc<State>,
}

impl Remux {
    #[rustfmt::skip]
    pub fn new(
        c: Arc<Conf>, 
        s: Arc<State>, 
        t: Arc<Controls>
    ) -> Self {
        Self {
            local: Arc::new(c.local),
            controls: t,
            state: s,
            conf: c,
        }
    }
    
    /// 处理数据
    ///
    /// 接收STUN编码的Bytes,
    /// 并返回任何可以响应的Bytes和目标地址
    /// Note: 内部隐含了未知编码的处理
    #[rustfmt::skip]
    pub async fn process<'a>(
        &self, 
        b: &'a [u8], 
        w: &'a mut BytesMut, 
        a: SocketAddr
    ) -> Result<Response<'a>> {
        let ctx = Context::from(&self, a);
        Ok(match Payload::try_from(b)? {
            Payload::ChannelData(x) => channel_data::process(ctx, x).await,
            Payload::Message(x) => Self::process_message(ctx, x, w).await?,
        })
    }
    
    /// 处理普通消息
    ///
    /// 处理通用STUN编码消息，
    /// 并返回对应的回复动作
    #[rustfmt::skip]
    #[inline(always)]
    async fn process_message<'a>(
        ctx: Context, 
        m: Message<'a>, 
        w: &'a mut BytesMut
    ) -> Result<Response<'a>> {
        match m.kind {
            Kind::BindingRequest => binding::process(ctx, m, w),
            Kind::AllocateRequest => allocate::process(ctx, m, w).await,
            Kind::CreatePermissionRequest => create_permission::process(ctx, m, w).await,
            Kind::SendIndication => indication::process(ctx, m, w).await,
            Kind::ChannelBindRequest => channel_bind::process(ctx, m, w).await,
            Kind::RefreshRequest => refresh::process(ctx, m, w).await,
            _ => Ok(None)
        }
    }
}

impl Context {
    fn from( h: &Remux, a: SocketAddr) -> Self {
        Self {
            controls: h.controls.clone(),
            state: h.state.clone(),
            local: h.local.clone(),
            conf: h.conf.clone(),
            addr: Arc::new(a),
        }
    }
    
    /// 获取认证信息
    ///
    /// 认证信息会缓存在实例内部，
    /// 当没有缓存的时候才会请求控制中心
    #[rustfmt::skip]
    pub async fn get_auth(&self, u: &str) -> Option<Arc<String>> {
        if let Some(a) = self.state.get_password(&self.addr).await {
            return Some(a)
        }

        let auth = match self.controls.auth(u, &self.addr).await {
            Ok(a) => a,
            Err(e) => { 
                log::warn!("controls auth err: {}", e); 
                return None 
            }
        };
        
        self.state.insert(self.addr.clone(), &auth).await;
        Some(Arc::new(auth.password))
    }
}
