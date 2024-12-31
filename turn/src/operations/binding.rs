use super::{Requet, Response, ResponseMethod};
use crate::{Observer, SOFTWARE};

use stun::{
    attribute::{MappedAddress, ResponseOrigin, Software, XorMappedAddress},
    Kind, MessageReader, MessageWriter, Method,
};

/// process binding request
///
/// [rfc8489](https://tools.ietf.org/html/rfc8489)
///
/// In the Binding request/response transaction, a Binding request is
/// sent from a STUN client to a STUN server.  When the Binding request
/// arrives at the STUN server, it may have passed through one or more
/// NATs between the STUN client and the STUN server (in Figure 1, there
/// are two such NATs).  As the Binding request message passes through a
/// NAT, the NAT will modify the source transport address (that is, the
/// source IP address and the source port) of the packet.  As a result,
/// the source transport address of the request received by the server
/// will be the public IP address and port created by the NAT closest to
/// the server.  This is called a "reflexive transport address".  The
/// STUN server copies that source transport address into an XOR-MAPPED-
/// ADDRESS attribute in the STUN Binding response and sends the Binding
/// response back to the STUN client.  As this packet passes back through
/// a NAT, the NAT will modify the destination transport address in the
/// IP header, but the transport address in the XOR-MAPPED-ADDRESS
/// attribute within the body of the STUN response will remain untouched.
/// In this way, the client can learn its reflexive transport address
/// allocated by the outermost NAT with respect to the STUN server.
pub fn process<'a, T: Observer>(req: Requet<'_, 'a, T, MessageReader<'_>>) -> Option<Response<'a>> {
    {
        let mut message =
            MessageWriter::extend(Method::Binding(Kind::Response), &req.message, req.bytes);

        message.append::<XorMappedAddress>(req.address.address);
        message.append::<MappedAddress>(req.address.address);
        message.append::<ResponseOrigin>(req.service.interface);
        message.append::<Software>(SOFTWARE);
        message.flush(None).ok()?;
    }

    Some(Response {
        method: ResponseMethod::Stun(Method::Binding(Kind::Response)),
        bytes: req.bytes,
        endpoint: None,
        relay: None,
    })
}
