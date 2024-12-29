use super::{Requet, Response};
use crate::{Observer, StunClass};

use stun::{
    attribute::{ChannelNumber, Error, ErrorCode, ErrorKind, Realm, XorPeerAddress},
    Kind, MessageReader, MessageWriter, Method,
};

/// return channel binding error response
#[inline(always)]
fn reject<'a, T: Observer>(
    req: Requet<'_, 'a, T, MessageReader<'_>>,
    err: ErrorKind,
) -> Option<Response<'a>> {
    {
        let mut message =
            MessageWriter::extend(Method::ChannelBind(Kind::Error), req.message, req.bytes);

        message.append::<ErrorCode>(Error::from(err));
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

/// return channel binding ok response
#[inline(always)]
fn resolve<'a, T: Observer>(
    req: Requet<'_, 'a, T, MessageReader<'_>>,
    digest: &[u8; 16],
) -> Option<Response<'a>> {
    {
        MessageWriter::extend(Method::ChannelBind(Kind::Response), req.message, req.bytes)
            .flush(Some(digest))
            .ok()?;
    }

    Some(Response {
        kind: StunClass::Message,
        bytes: req.bytes,
        interface: None,
        reject: false,
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
pub async fn process<'a, T: Observer>(
    req: Requet<'_, 'a, T, MessageReader<'_>>,
) -> Option<Response<'a>> {
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

    let (username, digest) = match req.auth().await {
        None => return reject(req, ErrorKind::Unauthorized),
        Some(it) => it,
    };

    if !req
        .service
        .sessions
        .bind_channel(&req.symbol, peer.port(), number)
    {
        return reject(req, ErrorKind::Forbidden);
    }

    req.service
        .observer
        .channel_bind(&req.symbol, username, number);
    resolve(req, &digest)
}
