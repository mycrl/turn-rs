use std::{net::SocketAddr, sync::Arc};

use bytes::BytesMut;
use rand::seq::IteratorRandom;

use super::{
    InterfaceAddr, Service, ServiceHandler,
    session::{DEFAULT_SESSION_LIFETIME, Identifier, SessionManager},
};

use crate::{
    codec::{
        DecodeResult, Decoder,
        channel_data::ChannelData,
        crypto::Password,
        message::{
            Message, MessageEncoder,
            attributes::{address::IpAddrExt, error::ErrorType, *},
            methods::*,
        },
    },
    service::Transport,
};

struct Request<'a, 'b, T, M>
where
    T: ServiceHandler,
{
    response_buffer: &'b mut BytesMut,
    state: &'a RouterState<T>,
    payload: &'a M,
}

impl<'a, 'b, T> Request<'a, 'b, T, Message<'a>>
where
    T: ServiceHandler,
{
    // Verify the IP address specified by the client in the request, such as the
    // peer address used when creating permissions and binding channels. Currently,
    // only peer addresses that are local addresses of the TURN server are allowed;
    // arbitrary addresses are not permitted.
    //
    // Allowing arbitrary addresses would pose security risks, such as enabling
    // the TURN server to forward data to any target.
    #[inline(always)]
    fn verify_ip(&self, address: &SocketAddr) -> bool {
        self.state
            .interfaces
            .iter()
            .any(|item| item.external.ip() == address.ip())
    }

    // The key for the HMAC depends on whether long-term or short-term
    // credentials are in use.  For long-term credentials, the key is 16
    // bytes:
    //
    // key = MD5(username ":" realm ":" SASLprep(password))
    //
    // That is, the 16-byte key is formed by taking the MD5 hash of the
    // result of concatenating the following five fields: (1) the username,
    // with any quotes and trailing nulls removed, as taken from the
    // USERNAME attribute (in which case SASLprep has already been applied);
    // (2) a single colon; (3) the realm, with any quotes and trailing nulls
    // removed; (4) a single colon; and (5) the password, with any trailing
    // nulls removed and after processing using SASLprep.  For example, if
    // the username was 'user', the realm was 'realm', and the password was
    // 'pass', then the 16-byte HMAC key would be the result of performing
    // an MD5 hash on the string 'user:realm:pass', the resulting hash being
    // 0x8493fbc53ba582fb4c044c456bdc40eb.
    //
    // For short-term credentials:
    //
    // key = SASLprep(password)
    //
    // where MD5 is defined in RFC 1321 [RFC1321] and SASLprep() is defined
    // in RFC 4013 [RFC4013].
    //
    // The structure of the key when used with long-term credentials
    // facilitates deployment in systems that also utilize SIP.  Typically,
    // SIP systems utilizing SIP's digest authentication mechanism do not
    // actually store the password in the database.  Rather, they store a
    // value called H(A1), which is equal to the key defined above.
    //
    // Based on the rules above, the hash used to construct MESSAGE-
    // INTEGRITY includes the length field from the STUN message header.
    // Prior to performing the hash, the MESSAGE-INTEGRITY attribute MUST be
    // inserted into the message (with dummy content).  The length MUST then
    // be set to point to the length of the message up to, and including,
    // the MESSAGE-INTEGRITY attribute itself, but excluding any attributes
    // after it.  Once the computation is performed, the value of the
    // MESSAGE-INTEGRITY attribute can be filled in, and the value of the
    // length in the STUN header can be set to its correct value -- the
    // length of the entire message.  Similarly, when validating the
    // MESSAGE-INTEGRITY, the length field should be adjusted to point to
    // the end of the MESSAGE-INTEGRITY attribute prior to calculating the
    // HMAC.  Such adjustment is necessary when attributes, such as
    // FINGERPRINT, appear after MESSAGE-INTEGRITY.
    #[inline(always)]
    async fn verify(&self) -> Option<(&str, Password)> {
        let username = self.payload.get::<UserName>()?;
        let algorithm = self
            .payload
            .get::<PasswordAlgorithm>()
            .unwrap_or(PasswordAlgorithm::Md5);

        let password = self
            .state
            .manager
            .get_password(&self.state.id, username, algorithm)
            .await?;

        if self.payload.verify(&password).is_err() {
            return None;
        }

        Some((username, password))
    }
}

/// The route result.
#[derive(Debug)]
pub struct RouteResult {
    /// if the method is None, the response is a channel data response
    pub method: Option<Method>,
    /// the relay target of the response
    pub relay: Option<Identifier>,
}

pub(crate) struct RouterState<T>
where
    T: ServiceHandler,
{
    pub id: Identifier,
    pub realm: String,
    pub software: String,
    pub manager: Arc<SessionManager<T>>,
    pub interfaces: Arc<Vec<InterfaceAddr>>,
    pub handler: T,
}

pub struct Router<T>
where
    T: ServiceHandler,
{
    state: RouterState<T>,
    decoder: Decoder,
}

impl<T> Router<T>
where
    T: ServiceHandler + Clone,
{
    pub fn new(service: &Service<T>, id: Identifier) -> Self {
        Self {
            decoder: Decoder::default(),
            state: RouterState {
                interfaces: service.interfaces.clone(),
                software: service.software.clone(),
                handler: service.handler.clone(),
                manager: service.manager.clone(),
                realm: service.realm.clone(),
                id,
            },
        }
    }

    pub async fn route(
        &mut self,
        bytes: &[u8],
        response_buffer: &mut BytesMut,
    ) -> Result<Option<RouteResult>, crate::codec::Error> {
        Ok(match self.decoder.decode(bytes)? {
            DecodeResult::ChannelData(channel) => channel_data(Request {
                state: &self.state,
                payload: &channel,
                response_buffer,
            }),
            DecodeResult::Message(message) => {
                let req = Request {
                    state: &self.state,
                    payload: &message,
                    response_buffer,
                };

                match req.payload.method() {
                    BINDING_REQUEST => binding(req),
                    ALLOCATE_REQUEST => allocate(req).await,
                    CREATE_PERMISSION_REQUEST => create_permission(req).await,
                    CHANNEL_BIND_REQUEST => channel_bind(req).await,
                    REFRESH_REQUEST => refresh(req).await,
                    SEND_INDICATION => indication(req),
                    _ => None,
                }
            }
        })
    }
}

fn reject<T>(req: Request<'_, '_, T, Message<'_>>, error: ErrorType) -> Option<RouteResult>
where
    T: ServiceHandler,
{
    let method = req.payload.method().error()?;

    {
        let mut message = MessageEncoder::extend(method, req.payload, req.response_buffer);

        message.append::<ErrorCode>(ErrorCode::from(error));

        if error == ErrorType::Unauthorized {
            message.append::<Realm>(&req.state.realm);
            message.append::<Nonce>(
                req.state
                    .manager
                    .get_session_or_default(&req.state.id)
                    .get_ref()?
                    .nonce(),
            );

            message.append::<PasswordAlgorithms>(vec![
                PasswordAlgorithm::Md5,
                PasswordAlgorithm::Sha256,
            ]);
        }

        message.flush(None).ok()?;
    }

    Some(RouteResult {
        method: Some(method),
        relay: None,
    })
}

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
fn binding<T>(req: Request<'_, '_, T, Message<'_>>) -> Option<RouteResult>
where
    T: ServiceHandler,
{
    {
        let mut message =
            MessageEncoder::extend(BINDING_RESPONSE, req.payload, req.response_buffer);

        message.append::<XorMappedAddress>(req.state.id.source);
        message.append::<MappedAddress>(req.state.id.source);
        message.append::<ResponseOrigin>(req.state.id.external);
        message.append::<Software>(&req.state.software);
        message.flush(None).ok()?;
    }

    Some(RouteResult {
        method: Some(BINDING_RESPONSE),
        relay: None,
    })
}

/// [rfc8489](https://tools.ietf.org/html/rfc8489)
///
/// In all cases, the server SHOULD only allocate ports from the range
/// 49152 - 65535 (the Dynamic and/or Private Port range [PORT-NUMBERS]),
/// unless the TURN server application knows, through some means not
/// specified here, that other applications running on the same host as
/// the TURN server application will not be impacted by allocating ports
/// outside this range.  This condition can often be satisfied by running
/// the TURN server application on a dedicated machine and/or by
/// arranging that any other applications on the machine allocate ports
/// before the TURN server application starts.  In any case, the TURN
/// server SHOULD NOT allocate ports in the range 0 - 1023 (the Well-
/// Known Port range) to discourage clients from using TURN to run
/// standard contexts.
async fn allocate<T>(req: Request<'_, '_, T, Message<'_>>) -> Option<RouteResult>
where
    T: ServiceHandler,
{
    let xor_relayed_ip = {
        let mut ip = req.state.id.external.ip();

        let request_transport = if let Some(it) = req.payload.get::<RequestedTransport>() {
            match it {
                RequestedTransport::Tcp => Transport::Tcp,
                RequestedTransport::Udp => Transport::Udp,
            }
        } else {
            return reject(req, ErrorType::BadRequest);
        };

        let request_family = req
            .payload
            .get::<RequestedAddressFamily>()
            .unwrap_or_else(|| ip.family());

        // If the requested transport protocol or address family does not match the
        // address assigned by the server, a different address must be selected.
        // Both conditions must be checked independently: even when request_family
        // is present, a transport mismatch also requires selecting a new interface.
        if request_transport != req.state.id.transport || request_family != ip.family() {
            if let Some(addr) = req
                .state
                .interfaces
                .iter()
                .filter(|addr| {
                    addr.transport == request_transport
                        && addr.external.ip().family() == request_family
                })
                .choose(&mut rand::rng())
            {
                ip = addr.external.ip();
            } else {
                return reject(
                    req,
                    if request_family != ip.family() {
                        ErrorType::AddressFamilyNotSupported
                    } else {
                        ErrorType::UnsupportedTransportAddress
                    },
                );
            }
        }

        ip
    };

    let Some((username, password)) = req.verify().await else {
        return reject(req, ErrorType::Unauthorized);
    };

    let lifetime = req.payload.get::<Lifetime>();

    let Some(port) = req.state.manager.allocate(&req.state.id, lifetime) else {
        return reject(req, ErrorType::AllocationQuotaReached);
    };

    req.state
        .handler
        .on_allocated(&req.state.id, username, port);

    {
        let mut message =
            MessageEncoder::extend(ALLOCATE_RESPONSE, req.payload, req.response_buffer);

        message.append::<XorRelayedAddress>(SocketAddr::new(xor_relayed_ip, port));
        message.append::<XorMappedAddress>(req.state.id.source);
        message.append::<Lifetime>(lifetime.unwrap_or(DEFAULT_SESSION_LIFETIME as u32));
        message.append::<Software>(&req.state.software);
        message.flush(Some(&password)).ok()?;
    }

    Some(RouteResult {
        method: Some(ALLOCATE_RESPONSE),
        relay: None,
    })
}

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
/// The port portion of each attribute is ignored and may be any arbitrary
/// value.
///
/// The server then responds with a CreatePermission success response.
/// There are no mandatory attributes in the success response.
///
/// NOTE: A server need not do anything special to implement idempotency of
/// CreatePermission requests over UDP using the "stateless stack approach".
/// Retransmitted CreatePermission requests will simply refresh the
/// permissions.
async fn create_permission<T>(req: Request<'_, '_, T, Message<'_>>) -> Option<RouteResult>
where
    T: ServiceHandler,
{
    let Some((username, password)) = req.verify().await else {
        return reject(req, ErrorType::Unauthorized);
    };

    let mut ports = Vec::with_capacity(15);
    for it in req.payload.get_all::<XorPeerAddress>() {
        if !req.verify_ip(&it) {
            return reject(req, ErrorType::PeerAddressFamilyMismatch);
        }

        ports.push(it.port());
    }

    if !req.state.manager.create_permission(&req.state.id, &ports) {
        return reject(req, ErrorType::Forbidden);
    }

    req.state
        .handler
        .on_create_permission(&req.state.id, username, &ports);

    {
        MessageEncoder::extend(CREATE_PERMISSION_RESPONSE, req.payload, req.response_buffer)
            .flush(Some(&password))
            .ok()?;
    }

    Some(RouteResult {
        method: Some(CREATE_PERMISSION_RESPONSE),
        relay: None,
    })
}

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
async fn channel_bind<T>(req: Request<'_, '_, T, Message<'_>>) -> Option<RouteResult>
where
    T: ServiceHandler,
{
    let Some(peer) = req.payload.get::<XorPeerAddress>() else {
        return reject(req, ErrorType::BadRequest);
    };

    if !req.verify_ip(&peer) {
        return reject(req, ErrorType::PeerAddressFamilyMismatch);
    }

    let Some(number) = req.payload.get::<ChannelNumber>() else {
        return reject(req, ErrorType::BadRequest);
    };

    if !(0x4000..=0xFFFF).contains(&number) {
        return reject(req, ErrorType::BadRequest);
    }

    let Some((username, password)) = req.verify().await else {
        return reject(req, ErrorType::Unauthorized);
    };

    if !req
        .state
        .manager
        .bind_channel(&req.state.id, peer.port(), number)
    {
        return reject(req, ErrorType::Forbidden);
    }

    req.state
        .handler
        .on_channel_bind(&req.state.id, username, number);

    {
        MessageEncoder::extend(CHANNEL_BIND_RESPONSE, req.payload, req.response_buffer)
            .flush(Some(&password))
            .ok()?;
    }

    Some(RouteResult {
        method: Some(CHANNEL_BIND_RESPONSE),
        relay: None,
    })
}

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
#[rustfmt::skip]
fn indication<T>(req: Request<'_, '_, T, Message<'_>>) -> Option<RouteResult>
where
    T: ServiceHandler,
{
    let peer = req.payload.get::<XorPeerAddress>()?;
    let data = req.payload.get::<Data>()?;

    let (local_port, relay) = req.state.manager.get_port_relay_address(&req.state.id, peer.port())?;

    {
        let mut message = MessageEncoder::extend(DATA_INDICATION, req.payload, req.response_buffer);

        message.append::<XorPeerAddress>(SocketAddr::new(req.state.id.external.ip(), local_port));
        message.append::<Data>(data);
        message.flush(None).ok()?;
    }

    Some(RouteResult {
        method: Some(DATA_INDICATION),
        relay: Some(relay),
    })
}

/// If the server receives a Refresh Request with a REQUESTED-ADDRESS-
/// FAMILY attribute and the attribute value does not match the address
/// family of the allocation, the server MUST reply with a 443 (Peer
/// Address Family Mismatch) Refresh error response.
///
/// The server computes a value called the "desired lifetime" as follows:
/// if the request contains a LIFETIME attribute and the attribute value
/// is zero, then the "desired lifetime" is zero.  Otherwise, if the
/// request contains a LIFETIME attribute, then the server computes the
/// minimum of the client's requested lifetime and the server's maximum
/// allowed lifetime.  If this computed value is greater than the default
/// lifetime, then the "desired lifetime" is the computed value.
/// Otherwise, the "desired lifetime" is the default lifetime.
///
/// Subsequent processing depends on the "desired lifetime" value:
///
/// * If the "desired lifetime" is zero, then the request succeeds and the
///   allocation is deleted.
///
/// * If the "desired lifetime" is non-zero, then the request succeeds and the
///   allocation's time-to-expiry is set to the "desired lifetime".
///
/// If the request succeeds, then the server sends a success response
/// containing:
///
/// * A LIFETIME attribute containing the current value of the time-to-expiry
///   timer.
///
/// NOTE: A server need not do anything special to implement
/// idempotency of Refresh requests over UDP using the "stateless
/// stack approach".  Retransmitted Refresh requests with a non-
/// zero "desired lifetime" will simply refresh the allocation.  A
/// retransmitted Refresh request with a zero "desired lifetime"
/// will cause a 437 (Allocation Mismatch) response if the
/// allocation has already been deleted, but the client will treat
/// this as equivalent to a success response (see below).
async fn refresh<T>(req: Request<'_, '_, T, Message<'_>>) -> Option<RouteResult>
where
    T: ServiceHandler,
{
    let Some((username, password)) = req.verify().await else {
        return reject(req, ErrorType::Unauthorized);
    };

    let lifetime = req
        .payload
        .get::<Lifetime>()
        .unwrap_or(DEFAULT_SESSION_LIFETIME as u32);
    if !req.state.manager.refresh(&req.state.id, lifetime) {
        return reject(req, ErrorType::AllocationMismatch);
    }

    req.state
        .handler
        .on_refresh(&req.state.id, username, lifetime);

    {
        let mut message =
            MessageEncoder::extend(REFRESH_RESPONSE, req.payload, req.response_buffer);

        message.append::<Lifetime>(lifetime);
        message.flush(Some(&password)).ok()?;
    }

    Some(RouteResult {
        method: Some(REFRESH_RESPONSE),
        relay: None,
    })
}

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
fn channel_data<T>(req: Request<'_, '_, T, ChannelData<'_>>) -> Option<RouteResult>
where
    T: ServiceHandler,
{
    let (relay_channel, relay) = req
        .state
        .manager
        .get_channel_relay_address(&req.state.id, req.payload.number())?;

    {
        ChannelData::new(relay_channel, req.payload.bytes()).encode(req.response_buffer);
    }

    Some(RouteResult {
        relay: Some(relay),
        method: None,
    })
}
