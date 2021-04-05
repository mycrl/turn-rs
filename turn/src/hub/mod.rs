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
    rpc::Rpc,
    config::Conf,
    state::State,
};

use std::{
    convert::TryFrom, 
    net::SocketAddr, 
    sync::Arc
};

use stun::{
    Kind, 
    Payload,
    MessageReader,
};

#[rustfmt::skip]
static SOFTWARE: &str = concat!(
    env!("CARGO_PKG_NAME"), "-",
    env!("CARGO_PKG_VERSION")
);

#[rustfmt::skip]
pub(crate) type Response<'a> = Option<(
    &'a [u8],
    Arc<SocketAddr>
)>;

/// message context
pub struct Context {
    pub conf: Arc<Conf>,
    pub state: Arc<State>,
    pub rpc: Arc<Rpc>,
    /// client socketaddr
    pub addr: Arc<SocketAddr>,
    /// local bind socketaddr
    pub local: Arc<SocketAddr>,
}

/// process udp message 
/// and return message + address.
pub struct Hub {
    rpc: Arc<Rpc>,
    local: Arc<SocketAddr>,
    conf: Arc<Conf>,
    state: Arc<State>,
}

impl Hub {
    pub fn new(c: &Arc<Conf>, s: &Arc<State>, t: &Arc<Rpc>) -> Self {
        Self {
            local: Arc::new(c.local),
            state: s.clone(),
            conf: c.clone(),
            rpc: t.clone(),
        }
    }
    
    /// process udp data
    ///
    /// receive STUN encoded Bytes, 
    /// and return any Bytes that can be responded to and the target address.
    /// Note: unknown message is not process.
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
    
    /// process stun message
    #[rustfmt::skip]
    #[inline(always)]
    async fn process_message<'a>(
        ctx: Context, 
        m: MessageReader<'a>, 
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
    fn from(h: &Hub, a: SocketAddr) -> Self {
        Self {
            rpc: h.rpc.clone(),
            state: h.state.clone(),
            local: h.local.clone(),
            conf: h.conf.clone(),
            addr: Arc::new(a),
        }
    }
    
    /// the authentication information will be cached inside the state, 
    /// and the control center will only be requested when there is no cache.
    #[rustfmt::skip]
    pub async fn get_password(&self, u: &str) -> Option<Arc<String>> {
        if let Some(a) = self.state.get_password(&self.addr).await {
            return Some(a)
        }

        let auth = match self.rpc.auth(u, &self.addr).await {
            Ok(a) => a,
            Err(e) => { 
                // when an error occurs, it is not passed to the caller, 
                // only a warning message is output here
                log::warn!("controls auth err: {}", e); 
                return None 
            }
        };
        
        self.state.insert(self.addr.clone(), &auth).await;
        Some(Arc::new(auth.password))
    }
}
