use super::{Requet, Response};
use crate::{Observer, StunClass, SOFTWARE};

use std::net::SocketAddr;

use stun::{
    attribute::{
        Error, ErrorCode, ErrorKind, Lifetime, Nonce, Realm, ReqeestedTransport, Software,
        XorMappedAddress, XorRelayedAddress,
    },
    Kind, MessageReader, MessageWriter, Method,
};

/// return allocate error response
#[inline(always)]
fn reject<'a, T: Observer>(
    req: Requet<'_, 'a, T, MessageReader<'_>>,
    err: ErrorKind,
) -> Option<Response<'a>> {
    {
        let mut message =
            MessageWriter::extend(Method::Allocate(Kind::Error), req.message, req.bytes);

        message.append::<ErrorCode>(Error::from(err));
        message.append::<Nonce>(&req.service.sessions.get_nonce(&req.symbol).get_ref()?.0);
        message.append::<Realm>(&req.service.realm);
        message.flush(None).ok()?;
    }

    Some(Response {
        kind: StunClass::Message,
        bytes: req.bytes,
        interface: None,
        reject: true,
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
    req: Requet<'_, 'a, T, MessageReader<'_>>,
    digest: &[u8; 16],
    port: u16,
) -> Option<Response<'a>> {
    {
        let mut message =
            MessageWriter::extend(Method::Allocate(Kind::Response), req.message, req.bytes);

        message.append::<XorRelayedAddress>(SocketAddr::new(req.service.external.ip(), port));
        message.append::<XorMappedAddress>(req.symbol.address);
        message.append::<Lifetime>(600);
        message.append::<Software>(SOFTWARE);
        message.flush(Some(digest)).ok()?;
    }

    Some(Response {
        kind: StunClass::Message,
        bytes: req.bytes,
        interface: None,
        reject: false,
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
pub async fn process<'a, T: Observer>(
    req: Requet<'_, 'a, T, MessageReader<'_>>,
) -> Option<Response<'a>> {
    if req.message.get::<ReqeestedTransport>().is_none() {
        return reject(req, ErrorKind::ServerError);
    }

    let (username, digest) = match req.auth().await {
        Some(it) => it,
        None => return reject(req, ErrorKind::Unauthorized),
    };

    let port = match req.service.sessions.allocate(req.symbol) {
        Some(it) => it,
        None => return reject(req, ErrorKind::AllocationQuotaReached),
    };

    req.service.observer.allocated(&req.symbol, username, port);
    resolve(req, &digest, port)
}
