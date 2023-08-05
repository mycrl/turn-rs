pub mod allocate;
pub mod binding;
pub mod channel_bind;
pub mod channel_data;
pub mod create_permission;
pub mod indication;
pub mod refresh;

use crate::{router::Router, Observer, StunClass};

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use bytes::BytesMut;
use faster_stun::attribute::*;
use faster_stun::*;

pub struct Env {
    pub interface: SocketAddr,
    pub realm: Arc<String>,
    pub router: Arc<Router>,
    pub external: Arc<SocketAddr>,
    pub externals: Arc<Vec<SocketAddr>>,
    pub observer: Arc<dyn Observer>,
}

/// process udp message
/// and return message + address.
pub struct Processor {
    env: Arc<Env>,
    decoder: Decoder,
    buf: BytesMut,
}

impl Processor {
    pub(crate) fn new(
        interface: SocketAddr,
        external: SocketAddr,
        externals: Arc<Vec<SocketAddr>>,
        realm: String,
        router: Arc<Router>,
        observer: Arc<dyn Observer>,
    ) -> Self {
        Self {
            decoder: Decoder::new(),
            buf: BytesMut::with_capacity(4096),
            env: Arc::new(Env {
                external: Arc::new(external),
                realm: Arc::new(realm),
                externals,
                interface,
                observer,
                router,
            }),
        }
    }

    /// process udp data
    ///
    /// receive STUN encoded Bytes,
    /// and return any Bytes that can be responded to and the target address.
    /// Note: unknown message is not process.
    ///
    /// In a typical configuration, a TURN client is connected to a private
    /// network [RFC1918] and, through one or more NATs, to the public
    /// Internet.  On the public Internet is a TURN server.  Elsewhere in the
    /// Internet are one or more peers with which the TURN client wishes to
    /// communicate.  These peers may or may not be behind one or more NATs.
    /// The client uses the server as a relay to send packets to these peers
    /// and to receive packets from these peers.
    ///
    /// ```text
    ///                                     Peer A
    ///                                     Server-Reflexive    +---------+
    ///                                    Transport Address   |         |
    ///                                      192.0.2.150:32102   |         |
    ///                                        |              /|         |
    ///                       TURN              |            / ^|  Peer A |
    ///    Client's           Server            |           /  ||         |
    ///    Host Transport     Transport         |         //   ||         |
    ///    Address            Address           |       //     |+---------+
    /// 198.51.100.2:49721  192.0.2.15:3478     |+-+  //     Peer A
    ///            |            |               ||N| /       Host Transport
    ///            |   +-+      |               ||A|/        Address
    ///            |   | |      |               v|T|     203.0.113.2:49582
    ///            |   | |      |               /+-+
    /// +---------+|   | |      |+---------+   /              +---------+
    /// |         ||   |N|      ||         | //               |         |
    /// | TURN    |v   | |      v| TURN    |/                 |         |
    /// | Client  |----|A|-------| Server  |------------------|  Peer B |
    /// |         |    | |^      |         |^                ^|         |
    /// |         |    |T||      |         ||                ||         |
    /// +---------+    | ||      +---------+|                |+---------+
    ///                | ||                 |                |
    ///                | ||                 |                |
    ///                +-+|                 |                |
    ///                   |                 |                |
    ///                   |                 |                |
    ///          Client's                   |             Peer B
    ///          Server-Reflexive     Relayed             Transport
    ///          Transport Address    Transport Address   Address
    ///          192.0.2.1:7000       192.0.2.15:50000    192.0.2.210:49191
    ///
    ///                                Figure 1
    /// ```
    ///
    /// Figure 1 shows a typical deployment.  In this figure, the TURN client
    /// and the TURN server are separated by a NAT, with the client on the
    /// private side and the server on the public side of the NAT.  This NAT
    /// is assumed to be a "bad" NAT; for example, it might have a mapping
    /// property of "address-and-port-dependent mapping" (see [RFC4787]).
    ///
    /// The client talks to the server from a (IP address, port) combination
    /// called the client's "host transport address".  (The combination of an
    /// IP address and port is called a "transport address".)
    ///
    /// The client sends TURN messages from its host transport address to a
    /// transport address on the TURN server that is known as the "TURN
    /// server transport address".  The client learns the TURN server
    /// transport address through some unspecified means (e.g.,
    /// configuration), and this address is typically used by many clients
    /// simultaneously.
    ///
    /// Since the client is behind a NAT, the server sees packets from the
    /// client as coming from a transport address on the NAT itself.  This
    /// address is known as the client's "server-reflexive transport
    /// address"; packets sent by the server to the client's server-reflexive
    /// transport address will be forwarded by the NAT to the client's host
    /// transport address.
    ///
    /// The client uses TURN commands to create and manipulate an ALLOCATION
    /// on the server.  An allocation is a data structure on the server.
    /// This data structure contains, amongst other things, the relayed
    /// transport address for the allocation.  The relayed transport address
    /// is the transport address on the server that peers can use to have the
    /// server relay data to the client.  An allocation is uniquely
    /// identified by its relayed transport address.
    ///
    /// Once an allocation is created, the client can send application data
    /// to the server along with an indication of to which peer the data is
    /// to be sent, and the server will relay this data to the intended peer.
    /// The client sends the application data to the server inside a TURN
    /// message; at the server, the data is extracted from the TURN message
    /// and sent to the peer in a UDP datagram.  In the reverse direction, a
    /// peer can send application data in a UDP datagram to the relayed
    /// transport address for the allocation; the server will then
    /// encapsulate this data inside a TURN message and send it to the client
    /// along with an indication of which peer sent the data.  Since the TURN
    /// message always contains an indication of which peer the client is
    /// communicating with, the client can use a single allocation to
    /// communicate with multiple peers.
    ///
    /// When the peer is behind a NAT, the client must identify the peer
    /// using its server-reflexive transport address rather than its host
    /// transport address.  For example, to send application data to Peer A
    /// in the example above, the client must specify 192.0.2.150:32102 (Peer
    /// A's server-reflexive transport address) rather than 203.0.113.2:49582
    /// (Peer A's host transport address).
    ///
    /// Each allocation on the server belongs to a single client and has
    /// either one or two relayed transport addresses that are used only by
    /// that allocation.  Thus, when a packet arrives at a relayed transport
    /// address on the server, the server knows for which client the data is
    /// intended.
    ///
    /// The client may have multiple allocations on a server at the same
    /// time.
    pub async fn process<'c, 'a: 'c>(
        &'a mut self,
        b: &'a [u8],
        addr: SocketAddr,
    ) -> Result<Option<Response<'c>>> {
        let ctx = Context {
            env: self.env.clone(),
            addr,
        };

        Ok(match self.decoder.decode(b)? {
            Payload::ChannelData(x) => channel_data::process(ctx, x),
            Payload::Message(x) => Self::message_process(ctx, x, &mut self.buf).await?,
        })
    }

    /// process stun message
    ///
    /// TURN is an extension to STUN.  All TURN messages, with the exception
    /// of the ChannelData message, are STUN-formatted messages.  All the
    /// base processing rules described in [RFC8489] apply to STUN-formatted
    /// messages.  This means that all the message-forming and message-
    /// processing descriptions in this document are implicitly prefixed with
    /// the rules of [RFC8489].
    ///
    /// [RFC8489] specifies an authentication mechanism called the "long-term
    /// credential mechanism".  TURN servers and clients MUST implement this
    /// mechanism, and the authentication options are discussed in
    /// Section 7.2.
    ///
    /// Note that the long-term credential mechanism applies only to requests
    /// and cannot be used to authenticate indications; thus, indications in
    /// TURN are never authenticated.  If the server requires requests to be
    /// authenticated, then the server's administrator MUST choose a realm
    /// value that will uniquely identify the username and password
    /// combination that the client must use, even if the client uses
    /// multiple servers under different administrations.  The server's
    /// administrator MAY choose to allocate a unique username to each
    /// client, or it MAY choose to allocate the same username to more than
    /// one client (for example, to all clients from the same department or
    /// company).  For each Allocate request, the server SHOULD generate a
    /// new random nonce when the allocation is first attempted following the
    /// randomness recommendations in [RFC4086] and SHOULD expire the nonce
    /// at least once every hour during the lifetime of the allocation.  The
    /// server uses the mechanism described in Section 9.2 of [RFC8489] to
    /// indicate that it supports [RFC8489].
    ///
    /// All requests after the initial Allocate must use the same username as
    /// that used to create the allocation to prevent attackers from
    /// hijacking the client's allocation.
    ///
    /// Specifically, if:
    ///
    /// * the server requires the use of the long-term credential mechanism, and;
    ///
    /// * a non-Allocate request passes authentication under this mechanism, and;
    ///
    /// * the 5-tuple identifies an existing allocation, but;
    ///
    /// * the request does not use the same username as used to create the
    ///   allocation,
    ///
    /// then the request MUST be rejected with a 441 (Wrong Credentials)
    /// error.
    ///
    /// When a TURN message arrives at the server from the client, the server
    /// uses the 5-tuple in the message to identify the associated
    /// allocation.  For all TURN messages (including ChannelData) EXCEPT an
    /// Allocate request, if the 5-tuple does not identify an existing
    /// allocation, then the message MUST either be rejected with a 437
    /// Allocation Mismatch error (if it is a request) or be silently ignored
    /// (if it is an indication or a ChannelData message).  A client
    /// receiving a 437 error response to a request other than Allocate MUST
    /// assume the allocation no longer exists.
    ///
    /// [RFC8489] defines a number of attributes, including the SOFTWARE and
    /// FINGERPRINT attributes.  The client SHOULD include the SOFTWARE
    /// attribute in all Allocate and Refresh requests and MAY include it in
    /// any other requests or indications.  The server SHOULD include the
    /// SOFTWARE attribute in all Allocate and Refresh responses (either
    /// success or failure) and MAY include it in other responses or
    /// indications.  The client and the server MAY include the FINGERPRINT
    /// attribute in any STUN-formatted messages defined in this document.
    ///
    /// TURN does not use the backwards-compatibility mechanism described in
    /// [RFC8489].
    ///
    /// TURN, as defined in this specification, supports both IPv4 and IPv6.
    /// IPv6 support in TURN includes IPv4-to-IPv6, IPv6-to-IPv6, and IPv6-
    /// to-IPv4 relaying.  When only a single address type is desired, the
    /// REQUESTED-ADDRESS-FAMILY attribute is used to explicitly request the
    /// address type the TURN server will allocate (e.g., an IPv4-only node
    /// may request the TURN server to allocate an IPv6 address).  If both
    /// IPv4 and IPv6 are desired, the single ADDITIONAL-ADDRESS-FAMILY
    /// attribute indicates a request to the server to allocate one IPv4 and
    /// one IPv6 relay address in a single Allocate request.  This saves
    /// local ports on the client and reduces the number of messages sent
    /// between the client and the TURN server.
    ///
    /// By default, TURN runs on the same ports as STUN: 3478 for TURN over
    /// UDP and TCP, and 5349 for TURN over (D)TLS.  However, TURN has its
    /// own set of Service Record (SRV) names: "turn" for UDP and TCP, and
    /// "turns" for (D)TLS.  Either the DNS resolution procedures or the
    /// ALTERNATE-SERVER procedures, both described in Section 7, can be used
    /// to run TURN on a different port.
    ///
    /// To ensure interoperability, a TURN server MUST support the use of UDP
    /// transport between the client and the server, and it SHOULD support
    /// the use of TCP, TLS-over-TCP, and DTLS-over-UDP transports.
    ///
    /// When UDP or DTLS-over-UDP transport is used between the client and
    /// the server, the client will retransmit a request if it does not
    /// receive a response within a certain timeout period.  Because of this,
    /// the server may receive two (or more) requests with the same 5-tuple
    /// and same transaction id.  STUN requires that the server recognize
    /// this case and treat the request as idempotent (see [RFC8489]).  Some
    /// implementations may choose to meet this requirement by remembering
    /// all received requests and the corresponding responses for 40 seconds
    /// (Section 6.3.1 of [RFC8489]).  Other implementations may choose to
    /// reprocess the request and arrange that such reprocessing returns
    /// essentially the same response.  To aid implementors who choose the
    /// latter approach (the so-called "stateless stack approach"), this
    /// specification includes some implementation notes on how this might be
    /// done.  Implementations are free to choose either approach or some
    /// other approach that gives the same results.
    ///
    /// To mitigate either intentional or unintentional denial-of-service
    /// attacks against the server by clients with valid usernames and
    /// passwords, it is RECOMMENDED that the server impose limits on both
    /// the number of allocations active at one time for a given username and
    /// on the amount of bandwidth those allocations can use.  The server
    /// should reject new allocations that would exceed the limit on the
    /// allowed number of allocations active at one time with a 486
    /// (Allocation Quota Exceeded) (see Section 7.2), and since UDP does not
    /// include a congestion control mechanism, it should discard application
    /// data traffic that exceeds the bandwidth quota.
    #[rustfmt::skip]
    #[inline(always)]
    async fn message_process<'c>(
        ctx: Context,
        m: MessageReader<'_, '_>,
        w: &'c mut BytesMut,
    ) -> Result<Option<Response<'c>>> {
        match m.method {
            Method::Binding(Kind::Request) => binding::process(ctx, m, w),
            Method::Allocate(Kind::Request) => allocate::process(ctx, m, w).await,
            Method::CreatePermission(Kind::Request) => create_permission::process(ctx, m, w).await,
            Method::ChannelBind(Kind::Request) => channel_bind::process(ctx, m, w).await,
            Method::Refresh(Kind::Request) => refresh::process(ctx, m, w).await,
            Method::SendIndication => indication::process(ctx, m, w).await,
            _ => Ok(None),
        }
    }
}

pub struct Response<'a> {
    pub data: &'a [u8],
    pub kind: StunClass,
    pub relay: Option<SocketAddr>,
    pub interface: Option<SocketAddr>,
}

impl<'a> Response<'a> {
    #[inline(always)]
    pub(crate) fn new(
        data: &'a [u8],
        kind: StunClass,
        relay: Option<SocketAddr>,
        interface: Option<SocketAddr>,
    ) -> Self {
        Self {
            data,
            kind,
            relay,
            interface,
        }
    }
}

pub struct Context {
    pub env: Arc<Env>,
    pub addr: SocketAddr,
}

/// The key for the HMAC depends on whether long-term or short-term
/// credentials are in use.  For long-term credentials, the key is 16
/// bytes:
///
/// key = MD5(username ":" realm ":" SASLprep(password))
///
/// That is, the 16-byte key is formed by taking the MD5 hash of the
/// result of concatenating the following five fields: (1) the username,
/// with any quotes and trailing nulls removed, as taken from the
/// USERNAME attribute (in which case SASLprep has already been applied);
/// (2) a single colon; (3) the realm, with any quotes and trailing nulls
/// removed; (4) a single colon; and (5) the password, with any trailing
/// nulls removed and after processing using SASLprep.  For example, if
/// the username was 'user', the realm was 'realm', and the password was
/// 'pass', then the 16-byte HMAC key would be the result of performing
/// an MD5 hash on the string 'user:realm:pass', the resulting hash being
/// 0x8493fbc53ba582fb4c044c456bdc40eb.
///
/// For short-term credentials:
///
/// key = SASLprep(password)
///
/// where MD5 is defined in RFC 1321 [RFC1321] and SASLprep() is defined
/// in RFC 4013 [RFC4013].
///
/// The structure of the key when used with long-term credentials
/// facilitates deployment in systems that also utilize SIP.  Typically,
/// SIP systems utilizing SIP's digest authentication mechanism do not
/// actually store the password in the database.  Rather, they store a
/// value called H(A1), which is equal to the key defined above.
///
/// Based on the rules above, the hash used to construct MESSAGE-
/// INTEGRITY includes the length field from the STUN message header.
/// Prior to performing the hash, the MESSAGE-INTEGRITY attribute MUST be
/// inserted into the message (with dummy content).  The length MUST then
/// be set to point to the length of the message up to, and including,
/// the MESSAGE-INTEGRITY attribute itself, but excluding any attributes
/// after it.  Once the computation is performed, the value of the
/// MESSAGE-INTEGRITY attribute can be filled in, and the value of the
/// length in the STUN header can be set to its correct value -- the
/// length of the entire message.  Similarly, when validating the
/// MESSAGE-INTEGRITY, the length field should be adjusted to point to
/// the end of the MESSAGE-INTEGRITY attribute prior to calculating the
/// HMAC.  Such adjustment is necessary when attributes, such as
/// FINGERPRINT, appear after MESSAGE-INTEGRITY.
#[inline(always)]
pub(crate) async fn verify_message<'a>(
    ctx: &Context,
    reader: &MessageReader<'a, '_>,
) -> Option<(&'a str, Arc<[u8; 16]>)> {
    let username = reader.get::<UserName>()?;
    let key = ctx
        .env
        .router
        .get_key(&ctx.addr, &ctx.env.interface, &ctx.env.external, username)
        .await?;

    reader.integrity(&key).ok()?;
    Some((username, key))
}

/// Check if the ip address belongs to the current turn server.
#[inline(always)]
pub(crate) fn ip_is_local(ctx: &Context, addr: &SocketAddr) -> bool {
    ctx.env.externals.iter().any(|item| item.ip() == addr.ip())
}
