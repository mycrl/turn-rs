use anyhow::Result;
use bytes::BytesMut;
use super::{ 
    Context, 
    Response
};

use std::{
    net::SocketAddr, 
    sync::Arc
};

use crate::payload::{
    Addr, 
    AttrKind, 
    ErrKind, 
    Error, 
    Kind, 
    Message, 
    Property
};

use crate::payload::ErrKind::{
    ServerError,
    Unauthorized,
};

/// 返回分配失败响应
#[inline(always)]
async fn reject<'a>(
    ctx: Context, 
    message: Message<'a>,
    w: &'a mut BytesMut,
    e: ErrKind, 
) -> Result<Response<'a>> {
    let nonce = ctx.state.get_nonce(&ctx.addr).await;
    let mut pack = message.extends(Kind::AllocateError);
    pack.append(Property::ErrorCode(Error::from(e)));
    pack.append(Property::Realm(&ctx.conf.realm));
    pack.append(Property::Nonce(&nonce));
    pack.try_into(w, None)?;
    Ok(Some((w, ctx.addr)))
}

/// 返回分配成功响应
///
/// NOTE: The use of randomized port assignments to avoid certain
/// types of attacks is described in [RFC6056].  It is RECOMMENDED
/// that a TURN server implement a randomized port assignment
/// algorithm from [RFC6056].  This is especially applicable to
/// servers that choose to pre-allocate a number of ports from the
/// underlying OS and then later assign them to allocations; for
/// example, a server may choose this technique to implement the
/// EVEN-PORT attribute.
#[inline(always)]
async fn resolve<'a>(
    ctx: &Context,
    message: &Message<'a>,
    u: &str,
    p: &str,
    port: u16,
    w: &'a mut BytesMut,
) -> Result<Response<'a>> {
    let alloc_addr = Arc::new(SocketAddr::new(ctx.local.ip(), port));
    let mut pack = message.extends(Kind::AllocateResponse);
    pack.append(Property::XorRelayedAddress(Addr(alloc_addr)));
    pack.append(Property::XorMappedAddress(Addr(ctx.addr.clone())));
    pack.append(Property::ResponseOrigin(Addr(ctx.local.clone())));
    pack.append(Property::Lifetime(600));
    pack.try_into(w, Some((u, &p, &ctx.conf.realm)))?;
    Ok(Some((w, ctx.addr.clone())))
}

/// 处理分配请求
///
/// [rfc8489](https://tools.ietf.org/html/rfc8489)
///
/// In all cases, the server SHOULD only allocate ports from the range
/// 49152 - 65535 (the Dynamic and/or Private Port range [PORT-NUMBERS]),
/// unless the TURN server application knows, through some means not
/// specified here, that other applications running on the same host as
/// the TURN server application will not be impacted by allocating ports
/// outside this range.  This condition can often be satisfied by running
/// the TURN server application on a dedicated machine and/or by
/// arranging that any other applications on the machine allocate ports
/// before the TURN server application starts.  In any case, the TURN
/// server SHOULD NOT allocate ports in the range 0 - 1023 (the Well-
/// Known Port range) to discourage clients from using TURN to run
/// standard services.
#[rustfmt::skip]
pub async fn process<'a>(ctx: Context, m: Message<'a>, w: &'a mut BytesMut) -> Result<Response<'a>> {
    let u = match m.get(AttrKind::UserName) {
        Some(Property::UserName(u)) => u,
        _ => return reject(ctx, m, w, Unauthorized).await,
    };

    if m.get(AttrKind::ReqeestedTransport).is_none() {
        return reject(ctx, m, w, ServerError).await
    }

    let key = match ctx.get_auth(u).await {
        None => return reject(ctx, m, w, Unauthorized).await,
        Some(p) => p,
    };

    let port = match ctx.state.alloc_port(ctx.addr.clone()).await {
        None => return reject(ctx, m, w, Unauthorized).await,
        Some(p) => p,
    };
    
    log::info!(
        "{:?} [{:?}] allocate port={}", 
        &ctx.addr,
        u,
        port,
    );

    match m.verify((u, &key, &ctx.conf.realm))? {
        false => reject(ctx, m, w, Unauthorized).await,
        true => resolve(&ctx, &m, u, &key, port, w).await,
    }
}
