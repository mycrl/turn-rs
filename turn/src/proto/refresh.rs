use bytes::BytesMut;
use anyhow::Result;
use super::{
    Context, 
    Response
};

use stun::{
    Kind, 
    MessageReader,
    MessageWriter
};

use stun::attribute::{
    ErrKind::Unauthorized,
    ErrKind,
    Error,
    ErrorCode,
    Lifetime,
    UserName
};

/// return refresh error response
#[inline(always)]
fn reject<'a>(
    ctx: Context, 
    m: MessageReader<'a>, 
    w: &'a mut BytesMut, 
    e: ErrKind
) -> Result<Response<'a>> {
    let mut pack = MessageWriter::derive(Kind::RefreshError, &m, w);
    pack.append::<ErrorCode>(Error::from(e));
    pack.try_into(None)?;
    Ok(Some((w, ctx.addr)))
}

/// return refresh ok response
#[inline(always)]
pub fn resolve<'a>(
    ctx: &Context, 
    m: &MessageReader<'a>, 
    lifetime: u32,
    u: &str,
    p: &str,
    w: &'a mut BytesMut
) -> Result<Response<'a>> {
    let mut pack = MessageWriter::derive(Kind::RefreshResponse, m , w);
    pack.append::<Lifetime>(lifetime);
    pack.try_into(Some((u, p, &ctx.conf.realm)))?;
    Ok(Some((w, ctx.addr.clone())))
}

/// process refresh request
///
/// If the server receives a Refresh Request with a REQUESTED-ADDRESS-
/// FAMILY attribute and the attribute value does not match the address
/// family of the allocation, the server MUST reply with a 443 (Peer
/// Address Family Mismatch) Refresh error response.
///
/// The server computes a value called the "desired lifetime" as follows:
/// if the request contains a LIFETIME attribute and the attribute value
/// is zero, then the "desired lifetime" is zero.  Otherwise, if the
/// request contains a LIFETIME attribute, then the server computes the
/// minimum of the client's requested lifetime and the server's maximum
/// allowed lifetime.  If this computed value is greater than the default
/// lifetime, then the "desired lifetime" is the computed value.
/// Otherwise, the "desired lifetime" is the default lifetime.
///
/// Subsequent processing depends on the "desired lifetime" value:
///
/// * If the "desired lifetime" is zero, then the request succeeds and
/// the allocation is deleted.
///
/// * If the "desired lifetime" is non-zero, then the request succeeds
/// and the allocation's time-to-expiry is set to the "desired
/// lifetime".
///
/// If the request succeeds, then the server sends a success response
/// containing:
///
/// * A LIFETIME attribute containing the current value of the time-to-
/// expiry timer.
///
/// NOTE: A server need not do anything special to implement
/// idempotency of Refresh requests over UDP using the "stateless
/// stack approach".  Retransmitted Refresh requests with a non-
/// zero "desired lifetime" will simply refresh the allocation.  A
/// retransmitted Refresh request with a zero "desired lifetime"
/// will cause a 437 (Allocation Mismatch) response if the
/// allocation has already been deleted, but the client will treat
/// this as equivalent to a success response (see below).
#[rustfmt::skip]
pub async fn process<'a>(ctx: Context, m: MessageReader<'a>, w: &'a mut BytesMut) -> Result<Response<'a>> {
    let u = match m.get::<UserName>() {
        Some(u) => u?,
        _ => return reject(ctx, m, w, Unauthorized),
    };

    let l = match m.get::<Lifetime>() {
        Some(l) => l?,
        _ => 600,
    };

    let key = match ctx.state.get_password(&ctx.addr, u).await {
        None => return reject(ctx, m, w, Unauthorized),
        Some(a) => a,
    };

    if m.integrity((u, &key, &ctx.conf.realm)).is_err() {
        return reject(ctx, m, w, Unauthorized);
    }
    
    log::info!(
        "{:?} [{:?}] refresh timeout={}", 
        &ctx.addr,
        u,
        l,
    );

    ctx.state.refresh(&ctx.addr, l).await;
    resolve(&ctx, &m, l, u, &key, w)
}
