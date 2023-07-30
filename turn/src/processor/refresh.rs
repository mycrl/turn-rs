use super::{verify_message, Context, Response};
use crate::StunClass;

use anyhow::Result;
use bytes::BytesMut;
use faster_stun::attribute::ErrKind::*;
use faster_stun::attribute::*;
use faster_stun::*;

/// return refresh error response
#[inline(always)]
fn reject<'a>(
    reader: MessageReader,
    bytes: &'a mut BytesMut,
    err: ErrKind,
) -> Result<Option<Response<'a>>> {
    let method = Method::Refresh(Kind::Error);
    let mut pack = MessageWriter::extend(method, &reader, bytes);
    pack.append::<ErrorCode>(Error::from(err));
    pack.flush(None)?;
    Ok(Some(Response::new(bytes, StunClass::Msg, None, None)))
}

/// return refresh ok response
#[inline(always)]
pub fn resolve<'a>(
    reader: &MessageReader,
    lifetime: u32,
    key: &[u8; 16],
    bytes: &'a mut BytesMut,
) -> Result<Option<Response<'a>>> {
    let method = Method::Refresh(Kind::Response);
    let mut pack = MessageWriter::extend(method, reader, bytes);
    pack.append::<Lifetime>(lifetime);
    pack.flush(Some(key))?;
    Ok(Some(Response::new(bytes, StunClass::Msg, None, None)))
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
pub async fn process<'a>(
    ctx: Context,
    reader: MessageReader<'_, '_>,
    bytes: &'a mut BytesMut,
) -> Result<Option<Response<'a>>> {
    let (username, key) = match verify_message(&ctx, &reader).await {
        None => return reject(reader, bytes, Unauthorized),
        Some(ret) => ret,
    };

    let time = reader.get::<Lifetime>().unwrap_or(600);
    ctx.env.observer.refresh(&ctx.addr, username, time);
    ctx.env.router.refresh(&ctx.addr, time);
    resolve(&reader, time, &key, bytes)
}
