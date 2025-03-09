use std::net::SocketAddr;

use super::{Observer, Requet, Response, ResponseMethod};

use crate::stun::{
    MessageEncoder, MessageRef,
    attribute::{
        Error, ErrorCode, ErrorKind, Lifetime, Nonce, Realm, ReqeestedTransport, Software, XorMappedAddress,
        XorRelayedAddress,
    },
    method::{ALLOCATE_ERROR, ALLOCATE_RESPONSE},
};

/// return allocate error response
#[inline(always)]
fn reject<'a, T: Observer>(req: Requet<'_, 'a, T, MessageRef<'_>>, err: ErrorKind) -> Option<Response<'a>> {
    {
        let mut message = MessageEncoder::extend(ALLOCATE_ERROR, req.message, req.bytes);
        message.append::<ErrorCode>(Error::from(err));
        message.append::<Nonce>(&req.service.sessions.get_nonce(&req.address).get_ref()?.0);
        message.append::<Realm>(&req.service.realm);
        message.flush(None).ok()?;
    }

    Some(Response {
        method: ResponseMethod::Stun(ALLOCATE_ERROR),
        bytes: req.bytes,
        endpoint: None,
        relay: None,
    })
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
fn resolve<'a, T: Observer>(
    req: Requet<'_, 'a, T, MessageRef<'_>>,
    integrity: &[u8; 16],
    port: u16,
) -> Option<Response<'a>> {
    {
        let mut message = MessageEncoder::extend(ALLOCATE_RESPONSE, req.message, req.bytes);
        message.append::<XorRelayedAddress>(SocketAddr::new(req.service.interface.ip(), port));
        message.append::<XorMappedAddress>(req.address.address);
        message.append::<Lifetime>(600);
        message.append::<Software>(&req.service.software);
        message.flush(Some(integrity)).ok()?;
    }

    Some(Response {
        method: ResponseMethod::Stun(ALLOCATE_RESPONSE),
        bytes: req.bytes,
        endpoint: None,
        relay: None,
    })
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
pub fn process<'a, T: Observer>(req: Requet<'_, 'a, T, MessageRef<'_>>) -> Option<Response<'a>> {
    if req.message.get::<ReqeestedTransport>().is_none() {
        return reject(req, ErrorKind::ServerError);
    }

    let (username, integrity) = match req.auth() {
        None => return reject(req, ErrorKind::Unauthorized),
        Some(it) => it,
    };

    let port = match req.service.sessions.allocate(req.address) {
        None => return reject(req, ErrorKind::AllocationQuotaReached),
        Some(it) => it,
    };

    req.service.observer.allocated(&req.address, username, port);
    resolve(req, &integrity, port)
}
