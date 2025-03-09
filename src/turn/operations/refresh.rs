use super::{Observer, Requet, Response, ResponseMethod};

use crate::stun::{
    MessageEncoder, MessageRef,
    attribute::{Error, ErrorCode, ErrorKind, Lifetime},
    method::{REFRESH_ERROR, REFRESH_RESPONSE},
};

/// return refresh error response
#[inline(always)]
fn reject<'a, T: Observer>(req: Requet<'_, 'a, T, MessageRef<'_>>, err: ErrorKind) -> Option<Response<'a>> {
    {
        let mut message = MessageEncoder::extend(REFRESH_ERROR, &req.message, req.bytes);
        message.append::<ErrorCode>(Error::from(err));
        message.flush(None).ok()?;
    }

    Some(Response {
        method: ResponseMethod::Stun(REFRESH_ERROR),
        bytes: req.bytes,
        endpoint: None,
        relay: None,
    })
}

/// return refresh ok response
#[inline(always)]
pub fn resolve<'a, T: Observer>(
    req: Requet<'_, 'a, T, MessageRef<'_>>,
    lifetime: u32,
    integrity: &[u8; 16],
) -> Option<Response<'a>> {
    {
        let mut message = MessageEncoder::extend(REFRESH_RESPONSE, &req.message, req.bytes);
        message.append::<Lifetime>(lifetime);
        message.flush(Some(integrity)).ok()?;
    }

    Some(Response {
        method: ResponseMethod::Stun(REFRESH_RESPONSE),
        bytes: req.bytes,
        endpoint: None,
        relay: None,
    })
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
/// * If the "desired lifetime" is zero, then the request succeeds and the
///   allocation is deleted.
///
/// * If the "desired lifetime" is non-zero, then the request succeeds and the
///   allocation's time-to-expiry is set to the "desired lifetime".
///
/// If the request succeeds, then the server sends a success response
/// containing:
///
/// * A LIFETIME attribute containing the current value of the time-to-expiry
///   timer.
///
/// NOTE: A server need not do anything special to implement
/// idempotency of Refresh requests over UDP using the "stateless
/// stack approach".  Retransmitted Refresh requests with a non-
/// zero "desired lifetime" will simply refresh the allocation.  A
/// retransmitted Refresh request with a zero "desired lifetime"
/// will cause a 437 (Allocation Mismatch) response if the
/// allocation has already been deleted, but the client will treat
/// this as equivalent to a success response (see below).
pub fn process<'a, T: Observer>(req: Requet<'_, 'a, T, MessageRef<'_>>) -> Option<Response<'a>> {
    let (username, integrity) = match req.auth() {
        None => return reject(req, ErrorKind::Unauthorized),
        Some(it) => it,
    };

    let lifetime = req.message.get::<Lifetime>().unwrap_or(600);
    if !req.service.sessions.refresh(&req.address, lifetime) {
        return reject(req, ErrorKind::AllocationMismatch);
    }

    req.service.observer.refresh(&req.address, username, lifetime);
    resolve(req, lifetime, &integrity)
}
