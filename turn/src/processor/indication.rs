use anyhow::Result;
use bytes::BytesMut;
use std::{
    net::SocketAddr,
    sync::Arc,
};

use super::{
    Context,
    Response,
};

use faster_stun::{
    MessageReader,
    MessageWriter,
    Method,
};

use faster_stun::attribute::{
    XorPeerAddress,
    Data,
};

/// process send indication request
///
/// When the server receives a Send indication, it processes as per
/// [Section 5](https://tools.ietf.org/html/rfc8656#section-5) plus
/// the specific rules mentioned here.
///
/// The message is first checked for validity.  The Send indication MUST
/// contain both an XOR-PEER-ADDRESS attribute and a DATA attribute.  If
/// one of these attributes is missing or invalid, then the message is
/// discarded.  Note that the DATA attribute is allowed to contain zero
/// bytes of data.
///
/// The Send indication may also contain the DONT-FRAGMENT attribute.  If
/// the server is unable to set the DF bit on outgoing UDP datagrams when
/// this attribute is present, then the server acts as if the DONT-
/// FRAGMENT attribute is an unknown comprehension-required attribute
/// (and thus the Send indication is discarded).
///
/// The server also checks that there is a permission installed for the
/// IP address contained in the XOR-PEER-ADDRESS attribute.  If no such
/// permission exists, the message is discarded.  Note that a Send
/// indication never causes the server to refresh the permission.
///
/// The server MAY impose restrictions on the IP address and port values
/// allowed in the XOR-PEER-ADDRESS attribute; if a value is not allowed,
/// the server silently discards the Send indication.
///
/// If everything is OK, then the server forms a UDP datagram as follows:
///
/// * the source transport address is the relayed transport address of
/// the allocation, where the allocation is determined by the 5-tuple
/// on which the Send indication arrived;
///
/// * the destination transport address is taken from the XOR-PEER-
/// ADDRESS attribute;
///
/// * the data following the UDP header is the contents of the value
/// field of the DATA attribute.
///
/// The handling of the DONT-FRAGMENT attribute (if present), is
/// described in Sections [14](https://tools.ietf.org/html/rfc8656#section-14)
/// and [15](https://tools.ietf.org/html/rfc8656#section-15).
///
/// The resulting UDP datagram is then sent to the peer.
pub fn process<'a, 'b, 'c>(
    ctx: Context,
    m: MessageReader<'a, 'b>,
    w: &'c mut BytesMut,
) -> Result<Response<'c>> {
    let peer = match m.get::<XorPeerAddress>() {
        None => return Ok(None),
        Some(x) => x,
    };

    if ctx.env.external.ip() != peer.ip() {
        return Ok(None);
    }

    let d = match m.get::<Data>() {
        None => return Ok(None),
        Some(x) => x,
    };

    let a = match ctx.env.router.get_port_bound(peer.port()) {
        None => return Ok(None),
        Some(a) => a,
    };

    let p = match ctx.env.router.get_bound_port(&ctx.addr, &a) {
        None => return Ok(None),
        Some(p) => p,
    };

    let method = Method::DataIndication;
    let s = Arc::new(SocketAddr::new(ctx.env.external.ip(), p));
    let mut pack = MessageWriter::extend(method, &m, w);
    pack.append::<XorPeerAddress>(*s.as_ref());
    pack.append::<Data>(d);
    pack.flush(None)?;
    Ok(Some((w, a)))
}
