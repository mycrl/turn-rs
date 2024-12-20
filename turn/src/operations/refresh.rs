use stun::{
    attribute::{ErrKind, Error, ErrorCode, Lifetime},
    Kind, MessageReader, MessageWriter, Method,
};

use super::{Requet, Response};
use crate::{Observer, StunClass};

/// return refresh error response
#[inline(always)]
fn reject<'a, T: Observer>(
    req: Requet<'_, 'a, T, MessageReader<'_>>,
    err: ErrKind,
) -> Option<Response<'a>> {
    {
        let mut message =
            MessageWriter::extend(Method::Refresh(Kind::Error), &req.message, req.bytes);

        message.append::<ErrorCode>(Error::from(err));
        message.flush(None).ok()?;
    }

    Some(Response {
        kind: StunClass::Message,
        bytes: req.bytes,
        interface: None,
        relay: None,
    })
}

/// return refresh ok response
#[inline(always)]
pub fn resolve<'a, T: Observer>(
    req: Requet<'_, 'a, T, MessageReader<'_>>,
    lifetime: u32,
    digest: &[u8; 16],
) -> Option<Response<'a>> {
    {
        let mut message =
            MessageWriter::extend(Method::Refresh(Kind::Response), &req.message, req.bytes);

        message.append::<Lifetime>(lifetime);
        message.flush(Some(digest)).ok()?;
    }

    Some(Response {
        kind: StunClass::Message,
        bytes: req.bytes,
        interface: None,
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
pub async fn process<'a, T: Observer>(
    req: Requet<'_, 'a, T, MessageReader<'_>>,
) -> Option<Response<'a>> {
    let (username, digest) = match req.auth().await {
        None => return reject(req, ErrKind::Unauthorized),
        Some(it) => it,
    };

    let lifetime = req.message.get::<Lifetime>().unwrap_or(600);
    if !req.service.sessions.refresh(&req.symbol, lifetime) {
        return reject(req, ErrKind::AllocationMismatch);
    }

    req.service
        .observer
        .refresh(&req.symbol, username, lifetime);
    resolve(req, lifetime, &digest)
}
