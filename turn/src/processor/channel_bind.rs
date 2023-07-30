use super::{verify_message, Context, Response};
use crate::StunClass;

use anyhow::Result;
use bytes::BytesMut;
use faster_stun::attribute::ErrKind::*;
use faster_stun::attribute::*;
use faster_stun::*;

/// return channel binding error response
#[inline(always)]
fn reject<'a>(
    ctx: Context,
    reader: MessageReader,
    bytes: &'a mut BytesMut,
    err: ErrKind,
) -> Result<Option<Response<'a>>> {
    let method = Method::ChannelBind(Kind::Error);
    let mut pack = MessageWriter::extend(method, &reader, bytes);
    pack.append::<ErrorCode>(Error::from(err));
    pack.append::<Realm>(&ctx.env.realm);
    pack.flush(None)?;
    Ok(Some(Response::new(bytes, StunClass::Msg, None, None)))
}

/// return channel binding ok response
#[inline(always)]
fn resolve<'a>(
    reader: &MessageReader,
    key: &[u8; 16],
    bytes: &'a mut BytesMut,
) -> Result<Option<Response<'a>>> {
    let method = Method::ChannelBind(Kind::Response);
    MessageWriter::extend(method, reader, bytes).flush(Some(key))?;
    Ok(Some(Response::new(bytes, StunClass::Msg, None, None)))
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
pub async fn process<'a>(
    ctx: Context,
    reader: MessageReader<'_, '_>,
    bytes: &'a mut BytesMut,
) -> Result<Option<Response<'a>>> {
    let peer = match reader.get::<XorPeerAddress>() {
        None => return reject(ctx, reader, bytes, BadRequest),
        Some(a) => a,
    };

    let number = match reader.get::<ChannelNumber>() {
        None => return reject(ctx, reader, bytes, BadRequest),
        Some(c) => c,
    };

    if ctx.env.external.ip() != peer.ip() {
        return reject(ctx, reader, bytes, Forbidden);
    }

    if !(0x4000..=0x7FFF).contains(&number) {
        return reject(ctx, reader, bytes, BadRequest);
    }

    let (username, key) = match verify_message(&ctx, &reader).await {
        None => return reject(ctx, reader, bytes, Unauthorized),
        Some(ret) => ret,
    };

    if ctx
        .env
        .router
        .bind_channel(&ctx.addr, peer.port(), number)
        .is_none()
    {
        return reject(ctx, reader, bytes, InsufficientCapacity);
    }

    ctx.env.observer.channel_bind(&ctx.addr, username, number);
    resolve(&reader, &key, bytes)
}
