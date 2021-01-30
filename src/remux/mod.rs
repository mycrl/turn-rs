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

static SOFTWARE: &str = concat!(
    "Mysticeti ",
    env!("CARGO_PKG_VERSION")
);

pub(crate) type Response<'a> = Option<(
    &'a [u8],
    Arc<SocketAddr>
)>;

/// 上下文
///
/// * `controls` 控制器
/// * `addr` 远程地址
/// * `local` 本地地址
/// * `state` 通道实例
/// * `conf` 配置
pub struct Context {
    pub conf: Arc<Conf>,
    pub state: Arc<State>,
    pub addr: Arc<SocketAddr>,
    pub controls: Arc<Controls>,
    pub local: Arc<SocketAddr>,
}

/// 解复用
///
/// * `controls` 控制器
/// * `local` 本地地址
/// * `state` 通道实例
/// * `conf` 配置
pub struct Remux {
    controls: Arc<Controls>,
    local: Arc<SocketAddr>,
    conf: Arc<Conf>,
    state: Arc<State>,
}

impl Remux {
    /// 创建实例
    ///
    /// # Example
    ///
    /// ```
    /// use super::{
    ///     controls::Controls,
    ///     conf as conf,
    ///     remux::Remux,
    ///     state::State,
    /// };
    /// 
    /// Remux::new(
    ///     conf::new(),
    ///     State::new(),
    ///     Controls::new(conf::new()),
    /// );
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// use super::{
    ///     controls::Controls,
    ///     conf as conf,
    ///     remux::Remux,
    ///     state::State,
    /// };
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     let remux = Remux::new(
    ///         conf::new(),
    ///         State::new(),
    ///         Controls::new(conf::new()),
    ///     );
    /// 
    ///     let addr = "127.0.0.1:8080".parse().unwrap();
    ///     if let Ok(Some((buf, a))) = remux.process(&[], addr).await {
    ///         // socket.send_to(buf, a.as_ref()).await;
    ///     }
    /// }
    /// ```
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
            Payload::Message(x) => Self::handle_message(ctx, x, w).await?,
        })
    }
    
    /// 处理普通消息
    ///
    /// 处理通用STUN编码消息，
    /// 并返回对应的回复动作
    #[rustfmt::skip]
    #[inline(always)]
    async fn handle_message<'a>(
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
    /// 创建上下文
    ///
    /// 从实例上派生出包含公共模块
    /// 和相关会话信息的上下文集合
    ///
    /// # Example
    ///
    /// ```
    /// use super::{
    ///     controls::Controls,
    ///     conf as conf,
    ///     remux::Context,
    ///     remux::Remux,
    ///     state::State,
    /// };
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     let remux = Remux::new(
    ///         conf::new(),
    ///         State::new(),
    ///         Controls::new(conf::new()),
    ///     );
    /// 
    ///     let addr = "127.0.0.1:8080".parse().unwrap();
    ///     Context::from(&remux, addr);
    /// }
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// use super::{
    ///     controls::Controls,
    ///     conf as conf,
    ///     remux::Context,
    ///     remux::Remux,
    ///     state::State,
    /// };
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     let remux = Remux::new(
    ///         conf::new(),
    ///         State::new(),
    ///         Controls::new(conf::new()),
    ///     );
    /// 
    ///     let addr = "127.0.0.1:8080".parse().unwrap();
    ///     let context = Context::from(&remux, addr);
    ///     if let Some(auth) = context.get_auth("panda").await {
    ///         // auth.password
    ///     }
    /// }
    /// ```
    pub async fn get_auth(&self, u: &str) -> Option<Arc<String>> {
        if let Some(a) = self.state.get_password(&self.addr).await {
            return Some(a)
        }

        let auth = match self.controls.auth(u, &self.addr).await {
            Err(e) => { log::warn!("controls auth err: {}", e); return None },
            Ok(a) => a,
        };
        
        self.state.insert(self.addr.clone(), &auth).await;
        Some(Arc::new(auth.password))
    }
}
