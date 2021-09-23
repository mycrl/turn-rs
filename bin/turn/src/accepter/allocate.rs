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

use stun::{
    MessageReader,
    MessageWriter,
    Kind
};

use stun::attribute::{
    Error,
    ErrKind,
    ErrorCode,
    Realm,
    Nonce,
    ReqeestedTransport,
    XorMappedAddress,
    XorRelayedAddress,
    ResponseOrigin,
    Lifetime,
    UserName
};

use stun::attribute::ErrKind::{
    Unauthorized,
    ServerError
};

/// return allocate error response
#[inline(always)]
async fn reject<'a, 'b>(
    ctx: Context, 
    m: MessageReader<'a, 'b>,
    w: &'a mut BytesMut,
    e: ErrKind, 
) -> Result<Response<'a>> {
    let nonce = ctx.state.get_nonce(&ctx.addr).await;
    let mut pack = MessageWriter::extend(Kind::AllocateError, &m, w);
    pack.append::<ErrorCode>(Error::from(e));
    pack.append::<Realm>(&ctx.conf.realm);
    pack.append::<Nonce>(&nonce);
    pack.flush(None)?;
    Ok(Some((w, ctx.addr)))
}

/// return allocate ok response
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
async fn resolve<'a, 'b>(
    ctx: &Context,
    m: &MessageReader<'a, 'b>,
    p: &[u8; 16],
    port: u16,
    w: &'a mut BytesMut,
) -> Result<Response<'a>> {
    let alloc_addr = Arc::new(SocketAddr::new(ctx.conf.external.ip(), port));
    let mut pack = MessageWriter::extend(Kind::AllocateResponse, m, w);
    pack.append::<XorRelayedAddress>(*alloc_addr.as_ref());
    pack.append::<XorMappedAddress>(*ctx.addr.as_ref());
    pack.append::<ResponseOrigin>(ctx.conf.external);
    pack.append::<Lifetime>(600);
    pack.flush(Some(p))?;
    Ok(Some((w, ctx.addr.clone())))
}

/// process allocate request
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
pub async fn process<'a, 'b>(ctx: Context, m: MessageReader<'a, 'b>, w: &'a mut BytesMut) -> Result<Response<'a>> {
    let u = match m.get::<UserName>() {
        Some(u) => u?,
        _ => return reject(ctx, m, w, Unauthorized).await,
    };

    if m.get::<ReqeestedTransport>().is_none() {
        return reject(ctx, m, w, ServerError).await
    }

    let key = match ctx.state.get_key(&ctx.addr, u).await {
        None => return reject(ctx, m, w, Unauthorized).await,
        Some(p) => p,
    };

    let port = match ctx.state.alloc_port(&ctx.addr).await {
        None => return reject(ctx, m, w, Unauthorized).await,
        Some(p) => p,
    };
    
    log::info!(
        "{:?} [{:?}] allocate port={}", 
        &ctx.addr,
        u,
        port,
    );

    match m.integrity(&key) {
        Err(_) => reject(ctx, m, w, Unauthorized).await,
        Ok(_) => resolve(&ctx, &m, &key, port, w).await,
    }
}
