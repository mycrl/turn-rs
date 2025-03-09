use super::{Observer, Requet, Response, ResponseMethod};

use crate::stun::{
    MessageEncoder, MessageRef,
    attribute::{ChannelNumber, Error, ErrorCode, ErrorKind, Realm, XorPeerAddress},
    method::{CHANNEL_BIND_ERROR, CHANNEL_BIND_RESPONSE},
};

/// return channel binding error response
#[inline(always)]
fn reject<'a, T: Observer>(req: Requet<'_, 'a, T, MessageRef<'_>>, err: ErrorKind) -> Option<Response<'a>> {
    {
        let mut message = MessageEncoder::extend(CHANNEL_BIND_ERROR, req.message, req.bytes);
        message.append::<ErrorCode>(Error::from(err));
        message.append::<Realm>(&req.service.realm);
        message.flush(None).ok()?;
    }

    Some(Response {
        method: ResponseMethod::Stun(CHANNEL_BIND_ERROR),
        bytes: req.bytes,
        endpoint: None,
        relay: None,
    })
}

/// return channel binding ok response
#[inline(always)]
fn resolve<'a, T: Observer>(req: Requet<'_, 'a, T, MessageRef<'_>>, integrity: &[u8; 16]) -> Option<Response<'a>> {
    {
        MessageEncoder::extend(CHANNEL_BIND_RESPONSE, req.message, req.bytes)
            .flush(Some(integrity))
            .ok()?;
    }

    Some(Response {
        method: ResponseMethod::Stun(CHANNEL_BIND_RESPONSE),
        bytes: req.bytes,
        endpoint: None,
        relay: None,
    })
}

/// process channel binding request
///
/// The server MAY impose restrictions on the IP address and port values
/// allowed in the XOR-PEER-ADDRESS attribute; if a value is not allowed,
/// the server rejects the request with a 403 (Forbidden) error.
///
/// If the request is valid, but the server is unable to fulfill the
/// request due to some capacity limit or similar, the server replies
/// with a 508 (Insufficient Capacity) error.
///
/// Otherwise, the server replies with a ChannelBind success response.
/// There are no required attributes in a successful ChannelBind
/// response.
///
/// If the server can satisfy the request, then the server creates or
/// refreshes the channel binding using the channel number in the
/// CHANNEL-NUMBER attribute and the transport address in the XOR-PEER-
/// ADDRESS attribute.  The server also installs or refreshes a
/// permission for the IP address in the XOR-PEER-ADDRESS attribute as
/// described in Section 9.
///
/// NOTE: A server need not do anything special to implement
/// idempotency of ChannelBind requests over UDP using the
/// "stateless stack approach".  Retransmitted ChannelBind requests
/// will simply refresh the channel binding and the corresponding
/// permission.  Furthermore, the client must wait 5 minutes before
/// binding a previously bound channel number or peer address to a
/// different channel, eliminating the possibility that the
/// transaction would initially fail but succeed on a
/// retransmission.
pub fn process<'a, T: Observer>(req: Requet<'_, 'a, T, MessageRef<'_>>) -> Option<Response<'a>> {
    let peer = match req.message.get::<XorPeerAddress>() {
        None => return reject(req, ErrorKind::BadRequest),
        Some(it) => it,
    };

    if !req.verify_ip(&peer) {
        return reject(req, ErrorKind::PeerAddressFamilyMismatch);
    }

    let number = match req.message.get::<ChannelNumber>() {
        None => return reject(req, ErrorKind::BadRequest),
        Some(it) => it,
    };

    if !(0x4000..=0x7FFF).contains(&number) {
        return reject(req, ErrorKind::BadRequest);
    }

    let (username, integrity) = match req.auth() {
        None => return reject(req, ErrorKind::Unauthorized),
        Some(it) => it,
    };

    if !req
        .service
        .sessions
        .bind_channel(&req.address, &req.service.endpoint, peer.port(), number)
    {
        return reject(req, ErrorKind::Forbidden);
    }

    req.service.observer.channel_bind(&req.address, username, number);
    resolve(req, &integrity)
}
