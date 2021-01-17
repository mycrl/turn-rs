use anyhow::Result;
use bytes::BytesMut;
use super::{
    Context, 
    Response
};

use crate::payload::{
    AttrKind, 
    ErrKind, 
    Error, 
    Kind, 
    Message, 
    Property
};

use crate::payload::ErrKind::{
    BadRequest,
    Unauthorized,
    AllocationMismatch,
};

/// 返回绑定失败响应
#[inline(always)]
fn reject<'a>(
    ctx: Context, 
    message: Message<'a>, 
    w: &'a mut BytesMut,
    e: ErrKind, 
) -> Result<Response<'a>> {
    let mut pack = message.extends(Kind::CreatePermissionError);
    pack.append(Property::ErrorCode(Error::from(e)));
    pack.append(Property::Realm(&ctx.conf.realm));
    pack.try_into(w, None)?;
    Ok(Some((w, ctx.addr)))
}

/// 返回绑定成功响应
///
/// 根据RFC并不需要任何属性
#[inline(always)]
fn resolve<'a>(
    ctx: &Context, 
    message: &Message, 
    u: &str, 
    p: &str, 
    w: &'a mut BytesMut
) -> Result<Response<'a>> {
    let pack = message.extends(Kind::ChannelBindResponse);
    pack.try_into(w, Some((u, p, &ctx.conf.realm)))?;
    Ok(Some((w, ctx.addr.clone())))
}

/// 处理频道绑定请求
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
pub async fn process<'a>(ctx: Context, m: Message<'a>, w: &'a mut BytesMut) -> Result<Response<'a>> {
    let u = match m.get(AttrKind::UserName) {
        Some(Property::UserName(u)) => u,
        _ => return reject(ctx, m, w, Unauthorized),
    };

    let c = match m.get(AttrKind::ChannelNumber) {
        Some(Property::ChannelNumber(c)) => *c,
        _ => return reject(ctx, m, w, BadRequest),
    };
    
    let p = match m.get(AttrKind::XorPeerAddress) {
        Some(Property::XorPeerAddress(a)) => a.addr().port(),
        _ => return reject(ctx, m, w, BadRequest)
    };

    if c < 0x4000 || c > 0x4FFF {
        return reject(ctx, m, w, BadRequest)
    }

    let key = match ctx.get_auth(u).await {
        None => return reject(ctx, m, w, Unauthorized),
        Some(a) => a,
    };

    if !m.verify((u, &key, &ctx.conf.realm))? {
        return reject(ctx, m, w, Unauthorized);
    }
    
    if !ctx.state.insert_channel(ctx.addr.clone(), p, c).await {
        return reject(ctx, m, w, AllocationMismatch);
    }
    
    log::info!(
        "{:?} [{:?}] bind channel={}", 
        &ctx.addr,
        u,
        c
    );

    resolve(&ctx, &m, u, &key, w)
}
