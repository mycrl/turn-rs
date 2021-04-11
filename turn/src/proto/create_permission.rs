use anyhow::Result;
use bytes::BytesMut;
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
    ErrKind, 
    ErrorCode,
    Error,
    Realm,
    UserName,
    XorPeerAddress
};

use stun::attribute::ErrKind::{
    BadRequest,
    Unauthorized,
    AllocationMismatch,
};

/// return create permission error response
#[inline(always)]
fn reject<'a>(
    ctx: Context, 
    m: MessageReader<'a>, 
    w: &'a mut BytesMut,
    e: ErrKind,
) -> Result<Response<'a>> {
    let mut pack = MessageWriter::derive(Kind::CreatePermissionError, &m, w);
    pack.append::<ErrorCode>(Error::from(e));
    pack.append::<Realm>(&ctx.conf.realm);
    pack.try_into(None)?;
    Ok(Some((w, ctx.addr)))
}

/// return create permission ok response
#[inline(always)]
fn resolve<'a>(
    ctx: &Context, 
    m: &MessageReader<'a>, 
    u: &str, 
    p: &str, 
    w: &'a mut BytesMut
) -> Result<Response<'a>> {
    MessageWriter::derive(Kind::CreatePermissionResponse, m, w)
        .try_into(Some((u, p, &ctx.conf.realm)))?;
    Ok(Some((w, ctx.addr.clone())))
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
/// The port portion of each attribute is ignored and may be any arbitrary value.
///
/// The server then responds with a CreatePermission success response.
/// There are no mandatory attributes in the success response.
///
/// > NOTE: A server need not do anything special to implement
/// idempotency of CreatePermission requests over UDP using the
/// "stateless stack approach".  Retransmitted CreatePermission
/// requests will simply refresh the permissions.
#[rustfmt::skip]
pub async fn process<'a>(ctx: Context, m: MessageReader<'a>, w: &'a mut BytesMut) -> Result<Response<'a>> {
    let u = match m.get::<UserName>() {
        Some(u) => u?,
        _ => return reject(ctx, m, w, Unauthorized),
    };

    let p = match m.get::<XorPeerAddress>() {
        Some(a) => a?.port(),
        _ => return reject(ctx, m, w, BadRequest)
    };

    let key = match ctx.state.get_password(&ctx.addr, u).await {
        None => return reject(ctx, m, w, Unauthorized),
        Some(a) => a,
    };

    if m.integrity((u, &key, &ctx.conf.realm)).is_err() {
        return reject(ctx, m, w, Unauthorized);
    }

    if !ctx.state.bind_peer(&ctx.addr, p).await {
        return reject(ctx, m, w, AllocationMismatch);
    }

    log::info!(
        "{:?} [{:?}] bind peer={}", 
        &ctx.addr,
        u,
        p,
    );

    resolve(&ctx, &m, u, &key, w)
}
