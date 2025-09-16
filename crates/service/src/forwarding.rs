use super::{
    Service, ServiceHandler,
    session::{Identifier, Session, SessionManager},
};

use std::{net::SocketAddr, sync::Arc};

use bytes::BytesMut;
use codec::{
    DecodeResult, Decoder,
    channel_data::ChannelData,
    message::{
        Message, MessageEncoder,
        attributes::{
            ChannelNumber, Data, ErrorCode, ErrorType, Lifetime, MappedAddress, Nonce, Realm,
            ReqeestedTransport, ResponseOrigin, Software, UserName, XorMappedAddress,
            XorPeerAddress, XorRelayedAddress,
        },
        methods::{
            ALLOCATE_REQUEST, ALLOCATE_RESPONSE, BINDING_REQUEST, BINDING_RESPONSE,
            CHANNEL_BIND_REQUEST, CHANNEL_BIND_RESPONSE, CREATE_PERMISSION_REQUEST,
            CREATE_PERMISSION_RESPONSE, DATA_INDICATION, Method as StunMethod, REFRESH_REQUEST,
            REFRESH_RESPONSE, SEND_INDICATION,
        },
    },
};

struct State<T>
where
    T: ServiceHandler,
{
    pub realm: String,
    pub software: String,
    pub manager: Arc<SessionManager<T>>,
    pub endpoint: SocketAddr,
    pub interface: SocketAddr,
    pub interfaces: Arc<Vec<SocketAddr>>,
    pub handler: T,
}

struct Inbound<'a, 'b, T, M>
where
    T: ServiceHandler + 'static,
{
    pub id: &'a Identifier,
    pub bytes: &'b mut BytesMut,
    pub state: &'a State<T>,
    pub payload: &'a M,
}

impl<'a, 'b, T> Inbound<'a, 'b, T, Message<'a>>
where
    T: ServiceHandler + 'static,
{
    #[inline(always)]
    pub fn verify_ip(&self, address: &SocketAddr) -> bool {
        self.state
            .interfaces
            .iter()
            .any(|item| item.ip() == address.ip())
    }

    #[inline(always)]
    pub fn credentials(&self) -> Option<(&str, [u8; 16])> {
        let username = self.payload.get::<UserName>()?;
        let password = self.state.manager.get_password(&self.id, username)?;

        self.payload.integrity(&password).ok()?;
        Some((username, password))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OutboundTarget {
    pub endpoint: Option<SocketAddr>,
    pub relay: Option<SocketAddr>,
}

#[derive(Debug)]
pub enum Outbound<'a> {
    Message {
        method: StunMethod,
        bytes: &'a [u8],
        target: OutboundTarget,
    },
    ChannelData {
        bytes: &'a [u8],
        target: OutboundTarget,
    },
}

#[derive(Debug)]
pub enum ForwardResult<'a> {
    Exceptional(codec::Error),
    Outbound(Outbound<'a>),
    None,
}

pub struct PacketForwarder<T>
where
    T: ServiceHandler + 'static,
{
    id: Identifier,
    state: State<T>,
    decoder: Decoder,
    bytes: BytesMut,
}

impl<T> PacketForwarder<T>
where
    T: ServiceHandler + Clone + 'static,
{
    pub fn new(service: &Service<T>, endpoint: SocketAddr, interface: SocketAddr) -> Self {
        Self {
            bytes: BytesMut::with_capacity(4096),
            decoder: Decoder::default(),
            id: Identifier {
                source: "0.0.0.0:0".parse().unwrap(),
                interface,
            },
            state: State {
                interfaces: service.interfaces.clone(),
                software: service.software.clone(),
                handler: service.handler.clone(),
                manager: service.manager.clone(),
                realm: service.realm.clone(),
                interface,
                endpoint,
            },
        }
    }

    pub fn forward<'a, 'b: 'a>(
        &'b mut self,
        bytes: &'b [u8],
        address: SocketAddr,
    ) -> ForwardResult<'a> {
        {
            self.id.source = address;
        }

        (match self.decoder.decode(bytes) {
            Ok(DecodeResult::ChannelData(channel)) => channel_data_route(
                bytes,
                Inbound {
                    id: &self.id,
                    state: &self.state,
                    bytes: &mut self.bytes,
                    payload: &channel,
                },
            ),
            Ok(DecodeResult::Message(message)) => {
                let req = Inbound {
                    id: &self.id,
                    state: &self.state,
                    bytes: &mut self.bytes,
                    payload: &message,
                };

                match req.payload.method() {
                    BINDING_REQUEST => binding(req),
                    ALLOCATE_REQUEST => allocate(req),
                    CREATE_PERMISSION_REQUEST => create_permission(req),
                    CHANNEL_BIND_REQUEST => channel_bind(req),
                    REFRESH_REQUEST => refresh(req),
                    SEND_INDICATION => indication(req),
                    _ => None,
                }
            }
            Err(e) => {
                return ForwardResult::Exceptional(e);
            }
        })
        .map(ForwardResult::Outbound)
        .unwrap_or(ForwardResult::None)
    }
}

#[rustfmt::skip]
fn reject<'a, T>(req: Inbound<'_, 'a, T, Message<'_>>, error: ErrorType) -> Option<Outbound<'a>>
where
    T: ServiceHandler,
{
    let method = req.payload.method().error()?;

    {
        let mut message = MessageEncoder::extend(method, req.payload, req.bytes);
        message.append::<ErrorCode>(ErrorCode::from(error));
        message.append::<Nonce>(req.state.manager.get_session_or_default(&req.id).get_ref()?.nonce());
        message.append::<Realm>(&req.state.realm);
        message.flush(None).ok()?;
    }

    Some(Outbound::Message {
        target: OutboundTarget::default(),
        bytes: req.bytes,
        method,
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
fn binding<'a, T>(req: Inbound<'_, 'a, T, Message<'_>>) -> Option<Outbound<'a>>
where
    T: ServiceHandler,
{
    {
        let mut message = MessageEncoder::extend(BINDING_RESPONSE, &req.payload, req.bytes);
        message.append::<XorMappedAddress>(req.id.source);
        message.append::<MappedAddress>(req.id.source);
        message.append::<ResponseOrigin>(req.state.interface);
        message.append::<Software>(&req.state.software);
        message.flush(None).ok()?;
    }

    Some(Outbound::Message {
        target: OutboundTarget::default(),
        method: BINDING_RESPONSE,
        bytes: req.bytes,
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
fn allocate<'a, T>(req: Inbound<'_, 'a, T, Message<'_>>) -> Option<Outbound<'a>>
where
    T: ServiceHandler,
{
    if req.payload.get::<ReqeestedTransport>().is_none() {
        return reject(req, ErrorType::ServerError);
    }

    let Some((username, password)) = req.credentials() else {
        return reject(req, ErrorType::Unauthorized);
    };

    let Some(port) = req.state.manager.allocate(req.id) else {
        return reject(req, ErrorType::AllocationQuotaReached);
    };

    req.state.handler.on_allocated(&req.id, username, port);

    {
        let mut message = MessageEncoder::extend(ALLOCATE_RESPONSE, req.payload, req.bytes);
        message.append::<XorRelayedAddress>(SocketAddr::new(req.state.interface.ip(), port));
        message.append::<XorMappedAddress>(req.id.source);
        message.append::<Lifetime>(600);
        message.append::<Software>(&req.state.software);
        message.flush(Some(&password)).ok()?;
    }

    Some(Outbound::Message {
        target: OutboundTarget::default(),
        method: ALLOCATE_RESPONSE,
        bytes: req.bytes,
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
fn channel_bind<'a, T>(req: Inbound<'_, 'a, T, Message<'_>>) -> Option<Outbound<'a>>
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

    if !(0x4000..=0x7FFF).contains(&number) {
        return reject(req, ErrorType::BadRequest);
    }

    let Some((username, password)) = req.credentials() else {
        return reject(req, ErrorType::Unauthorized);
    };

    if !req
        .state
        .manager
        .bind_channel(&req.id, &req.state.endpoint, peer.port(), number)
    {
        return reject(req, ErrorType::Forbidden);
    }

    req.state.handler.on_channel_bind(&req.id, username, number);

    {
        MessageEncoder::extend(CHANNEL_BIND_RESPONSE, req.payload, req.bytes)
            .flush(Some(&password))
            .ok()?;
    }

    Some(Outbound::Message {
        target: OutboundTarget::default(),
        method: CHANNEL_BIND_RESPONSE,
        bytes: req.bytes,
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
fn create_permission<'a, T>(req: Inbound<'_, 'a, T, Message<'_>>) -> Option<Outbound<'a>>
where
    T: ServiceHandler,
{
    let Some((username, password)) = req.credentials() else {
        return reject(req, ErrorType::Unauthorized);
    };

    let mut ports = Vec::with_capacity(15);
    for it in req.payload.get_all::<XorPeerAddress>() {
        if !req.verify_ip(&it) {
            return reject(req, ErrorType::PeerAddressFamilyMismatch);
        }

        ports.push(it.port());
    }

    if !req
        .state
        .manager
        .create_permission(&req.id, &req.state.endpoint, &ports)
    {
        return reject(req, ErrorType::Forbidden);
    }

    req.state
        .handler
        .on_create_permission(&req.id, username, &ports);

    {
        MessageEncoder::extend(CREATE_PERMISSION_RESPONSE, req.payload, req.bytes)
            .flush(Some(&password))
            .ok()?;
    }

    Some(Outbound::Message {
        method: CREATE_PERMISSION_RESPONSE,
        target: OutboundTarget::default(),
        bytes: req.bytes,
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
fn indication<'a, T>(req: Inbound<'_, 'a, T, Message<'_>>) -> Option<Outbound<'a>>
where
    T: ServiceHandler,
{
    let peer = req.payload.get::<XorPeerAddress>()?;
    let data = req.payload.get::<Data>()?;

    if let Some(Session::Authenticated { allocate_port, .. }) =
        req.state.manager.get_session(&req.id).get_ref()
    {
        if let Some(local_port) = *allocate_port {
            let relay = req.state.manager.get_relay_address(&req.id, peer.port())?;

            {
                let mut message = MessageEncoder::extend(DATA_INDICATION, &req.payload, req.bytes);
                message.append::<XorPeerAddress>(SocketAddr::new(req.state.interface.ip(), local_port));
                message.append::<Data>(data);
                message.flush(None).ok()?;
            }

            return Some(Outbound::Message {
                method: DATA_INDICATION,
                bytes: req.bytes,
                target: OutboundTarget {
                    relay: Some(relay.source),
                    endpoint: if req.state.endpoint != relay.endpoint {
                        Some(relay.endpoint)
                    } else {
                        None
                    },
                },
            });
        }
    }

    None
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
fn refresh<'a, T>(req: Inbound<'_, 'a, T, Message<'_>>) -> Option<Outbound<'a>>
where
    T: ServiceHandler,
{
    let Some((username, password)) = req.credentials() else {
        return reject(req, ErrorType::Unauthorized);
    };

    let lifetime = req.payload.get::<Lifetime>().unwrap_or(600);
    if !req.state.manager.refresh(&req.id, lifetime) {
        return reject(req, ErrorType::AllocationMismatch);
    }

    req.state.handler.on_refresh(&req.id, username, lifetime);

    {
        let mut message = MessageEncoder::extend(REFRESH_RESPONSE, &req.payload, req.bytes);
        message.append::<Lifetime>(lifetime);
        message.flush(Some(&password)).ok()?;
    }

    Some(Outbound::Message {
        target: OutboundTarget::default(),
        method: REFRESH_RESPONSE,
        bytes: req.bytes,
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
fn channel_data_route<'a, T>(
    bytes: &'a [u8],
    req: Inbound<'_, 'a, T, ChannelData<'_>>,
) -> Option<Outbound<'a>>
where
    T: ServiceHandler,
{
    let Some(relay) = req
        .state
        .manager
        .get_channel_relay_address(&req.id, req.payload.number())
    else {
        return None;
    };

    Some(Outbound::ChannelData {
        bytes,
        target: OutboundTarget {
            relay: Some(relay.source),
            endpoint: if req.state.endpoint != relay.endpoint {
                Some(relay.endpoint)
            } else {
                None
            },
        },
    })
}
