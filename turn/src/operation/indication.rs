use std::net::SocketAddr;

use super::{Requet, Response};
use crate::{Observer, StunClass};

use stun::{
    attribute::{Data, XorPeerAddress},
    MessageReader, MessageWriter, Method,
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
/// * the source transport address is the relayed transport address of the
///   allocation, where the allocation is determined by the 5-tuple on which the
///   Send indication arrived;
///
/// * the destination transport address is taken from the XOR-PEER-ADDRESS
///   attribute;
///
/// * the data following the UDP header is the contents of the value field of
///   the DATA attribute.
///
/// The handling of the DONT-FRAGMENT attribute (if present), is
/// described in Sections [14](https://tools.ietf.org/html/rfc8656#section-14)
/// and [15](https://tools.ietf.org/html/rfc8656#section-15).
///
/// The resulting UDP datagram is then sent to the peer.
pub fn process<'a, T: Observer>(req: Requet<'_, 'a, T, MessageReader<'_>>) -> Option<Response<'a>> {
    let peer = req.message.get::<XorPeerAddress>()?;
    if !req.ip_is_local(&peer) {
        return None;
    }

    let data = req.message.get::<Data>()?;
    let relay = req.service.state.get_port_addr(peer.port())?;
    let port = req.service.state.get_addr_port(&req.address)?;
    let interface = req.service.state.get_interface(&relay)?;

    {
        let mut message = MessageWriter::extend(Method::DataIndication, &req.message, req.bytes);
        message.append::<XorPeerAddress>(SocketAddr::new(interface.external.ip(), port));
        message.append::<Data>(data);
        message.flush(None).ok()?;
    }

    Some(Response {
        interface: (req.service.interface != interface.addr).then(|| interface.addr),
        kind: StunClass::Message,
        relay: Some(relay),
        bytes: req.bytes,
    })
}
