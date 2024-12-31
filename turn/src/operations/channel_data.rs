use super::{Requet, Response, ResponseMethod};
use crate::Observer;

use stun::ChannelData;

/// process channel data
///
/// If the ChannelData message is received on a channel that is not bound
/// to any peer, then the message is silently discarded.
///
/// On the client, it is RECOMMENDED that the client discard the
/// ChannelData message if the client believes there is no active
/// permission towards the peer.  On the server, the receipt of a
/// ChannelData message MUST NOT refresh either the channel binding or
/// the permission towards the peer.
///
/// On the server, if no errors are detected, the server relays the
/// application data to the peer by forming a UDP datagram as follows:
///
/// * the source transport address is the relayed transport address of the
///   allocation, where the allocation is determined by the 5-tuple on which the
///   ChannelData message arrived;
///
/// * the destination transport address is the transport address to which the
///   channel is bound;
///
/// * the data following the UDP header is the contents of the data field of the
///   ChannelData message.
///
/// The resulting UDP datagram is then sent to the peer.  Note that if
/// the Length field in the ChannelData message is 0, then there will be
/// no data in the UDP datagram, but the UDP datagram is still formed and
/// sent [(Section 4.1 of [RFC6263])](https://tools.ietf.org/html/rfc6263#section-4.1).
pub fn process<'a, T: Observer>(
    bytes: &'a [u8],
    req: Requet<'_, 'a, T, ChannelData<'a>>,
) -> Option<Response<'a>> {
    let relay = req
        .service
        .sessions
        .get_channel_relay_address(&req.address, req.message.number)?;

    Some(Response {
        method: ResponseMethod::ChannelData,
        endpoint: if req.service.endpoint != relay.endpoint {
            Some(relay.endpoint)
        } else {
            None
        },
        relay: Some(relay.address),
        bytes,
    })
}
