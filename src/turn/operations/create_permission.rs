use super::{Observer, Requet, Response, ResponseMethod};

use crate::stun::{
    MessageEncoder, MessageRef,
    attribute::{Error, ErrorCode, ErrorKind, Realm, Software, XorPeerAddress},
    method::{CREATE_PERMISSION_ERROR, CREATE_PERMISSION_RESPONSE},
};

/// return create permission error response
#[inline(always)]
fn reject<'a, T: Observer>(req: Requet<'_, 'a, T, MessageRef<'_>>, err: ErrorKind) -> Option<Response<'a>> {
    {
        let mut message = MessageEncoder::extend(CREATE_PERMISSION_ERROR, req.message, req.bytes);
        message.append::<ErrorCode>(Error::from(err));
        message.append::<Realm>(&req.service.realm);
        message.flush(None).ok()?;
    }

    Some(Response {
        method: ResponseMethod::Stun(CREATE_PERMISSION_ERROR),
        bytes: req.bytes,
        endpoint: None,
        relay: None,
    })
}

/// return create permission ok response
#[inline(always)]
fn resolve<'a, T: Observer>(req: Requet<'_, 'a, T, MessageRef<'_>>, integrity: &[u8; 16]) -> Option<Response<'a>> {
    {
        let mut message = MessageEncoder::extend(CREATE_PERMISSION_RESPONSE, req.message, req.bytes);
        message.append::<Software>(&req.service.software);
        message.flush(Some(integrity)).ok()?;
    }

    Some(Response {
        method: ResponseMethod::Stun(CREATE_PERMISSION_RESPONSE),
        bytes: req.bytes,
        endpoint: None,
        relay: None,
    })
}

/// process create permission request
///
/// [rfc8489](https://tools.ietf.org/html/rfc8489)
///
/// When the server receives the CreatePermission request, it processes
/// as per [Section 5](https://tools.ietf.org/html/rfc8656#section-5)
/// plus the specific rules mentioned here.
///
/// The message is checked for validity.  The CreatePermission request
/// MUST contain at least one XOR-PEER-ADDRESS attribute and MAY contain
/// multiple such attributes.  If no such attribute exists, or if any of
/// these attributes are invalid, then a 400 (Bad Request) error is
/// returned.  If the request is valid, but the server is unable to
/// satisfy the request due to some capacity limit or similar, then a 508
/// (Insufficient Capacity) error is returned.
///
/// If an XOR-PEER-ADDRESS attribute contains an address of an address
/// family that is not the same as that of a relayed transport address
/// for the allocation, the server MUST generate an error response with
/// the 443 (Peer Address Family Mismatch) response code.
///
/// The server MAY impose restrictions on the IP address allowed in the
/// XOR-PEER-ADDRESS attribute; if a value is not allowed, the server
/// rejects the request with a 403 (Forbidden) error.
///
/// If the message is valid and the server is capable of carrying out the
/// request, then the server installs or refreshes a permission for the
/// IP address contained in each XOR-PEER-ADDRESS attribute as described
/// in [Section 9](https://tools.ietf.org/html/rfc8656#section-9).  
/// The port portion of each attribute is ignored and may be any arbitrary
/// value.
///
/// The server then responds with a CreatePermission success response.
/// There are no mandatory attributes in the success response.
///
/// > NOTE: A server need not do anything special to implement idempotency of
/// > CreatePermission requests over UDP using the "stateless stack approach".
/// > Retransmitted CreatePermission requests will simply refresh the
/// > permissions.
pub fn process<'a, T: Observer>(req: Requet<'_, 'a, T, MessageRef<'_>>) -> Option<Response<'a>> {
    let (username, integrity) = match req.auth() {
        None => return reject(req, ErrorKind::Unauthorized),
        Some(it) => it,
    };

    let mut ports = Vec::with_capacity(15);
    for it in req.message.get_all::<XorPeerAddress>() {
        if !req.verify_ip(&it) {
            return reject(req, ErrorKind::PeerAddressFamilyMismatch);
        }

        ports.push(it.port());
    }

    if !req
        .service
        .sessions
        .create_permission(&req.address, &req.service.endpoint, &ports)
    {
        return reject(req, ErrorKind::Forbidden);
    }

    req.service.observer.create_permission(&req.address, username, &ports);
    resolve(req, &integrity)
}
