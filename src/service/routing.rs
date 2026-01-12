use std::{net::SocketAddr, sync::Arc};

use bytes::BytesMut;

use super::{
    Service, ServiceHandler,
    session::{DEFAULT_SESSION_LIFETIME, Identifier, Session, SessionManager},
};

use crate::codec::{
    DecodeResult, Decoder,
    channel_data::ChannelData,
    crypto::Password,
    message::{
        Message, MessageEncoder,
        attributes::{error::ErrorType, *},
        methods::*,
    },
};

struct Request<'a, 'b, T, M>
where
    T: ServiceHandler,
{
    id: &'a Identifier,
    encode_buffer: &'b mut BytesMut,
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
            .any(|item| item.ip() == address.ip())
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
            .get_password(self.id, username, algorithm)
            .await?;

        if self.payload.verify(&password).is_err() {
            return None;
        }

        Some((username, password))
    }
}

/// The target of the response.
#[derive(Debug, Clone, Copy, Default)]
pub struct Target {
    pub endpoint: Option<SocketAddr>,
    pub relay: Option<SocketAddr>,
}

/// The response.
#[derive(Debug)]
pub struct Response<'a> {
    /// if the method is None, the response is a channel data response
    pub method: Option<Method>,
    /// the bytes of the response
    pub bytes: &'a [u8],
    /// the target of the response
    pub target: Target,
}

pub(crate) struct RouterState<T>
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

pub struct Router<T>
where
    T: ServiceHandler,
{
    current_id: Identifier,
    state: RouterState<T>,
    decoder: Decoder,
    bytes: BytesMut,
}

impl<T> Router<T>
where
    T: ServiceHandler + Clone,
{
    pub fn new(service: &Service<T>, endpoint: SocketAddr, interface: SocketAddr) -> Self {
        Self {
            bytes: BytesMut::with_capacity(4096),
            decoder: Decoder::default(),
            // This is a placeholder address that will be updated on first route call
            current_id: Identifier::new(
                "0.0.0.0:0"
                    .parse()
                    .expect("Failed to parse placeholder address"),
                interface,
            ),
            state: RouterState {
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

    pub async fn route<'a, 'b: 'a>(
        &'b mut self,
        bytes: &'b [u8],
        address: SocketAddr,
    ) -> Result<Option<Response<'a>>, crate::codec::Error> {
        {
            *self.current_id.source_mut() = address;
        }

        Ok(match self.decoder.decode(bytes)? {
            DecodeResult::ChannelData(channel) => channel_data(
                bytes,
                Request {
                    id: &self.current_id,
                    state: &self.state,
                    encode_buffer: &mut self.bytes,
                    payload: &channel,
                },
            ),
            DecodeResult::Message(message) => {
                let req = Request {
                    id: &self.current_id,
                    state: &self.state,
                    encode_buffer: &mut self.bytes,
                    payload: &message,
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

fn reject<'a, T>(req: Request<'_, 'a, T, Message<'_>>, error: ErrorType) -> Option<Response<'a>>
where
    T: ServiceHandler,
{
    let method = req.payload.method().error()?;

    {
        let mut message = MessageEncoder::extend(method, req.payload, req.encode_buffer);
        message.append::<ErrorCode>(ErrorCode::from(error));

        if error == ErrorType::Unauthorized {
            message.append::<Realm>(&req.state.realm);
            message.append::<Nonce>(
                req.state
                    .manager
                    .get_session_or_default(req.id)
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

    Some(Response {
        target: Target::default(),
        bytes: req.encode_buffer,
        method: Some(method),
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
fn binding<'a, T>(req: Request<'_, 'a, T, Message<'_>>) -> Option<Response<'a>>
where
    T: ServiceHandler,
{
    {
        let mut message = MessageEncoder::extend(BINDING_RESPONSE, req.payload, req.encode_buffer);
        message.append::<XorMappedAddress>(req.id.source());
        message.append::<MappedAddress>(req.id.source());
        message.append::<ResponseOrigin>(req.state.interface);
        message.append::<Software>(&req.state.software);
        message.flush(None).ok()?;
    }

    Some(Response {
        method: Some(BINDING_RESPONSE),
        target: Target::default(),
        bytes: req.encode_buffer,
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
async fn allocate<'a, T>(req: Request<'_, 'a, T, Message<'_>>) -> Option<Response<'a>>
where
    T: ServiceHandler,
{
    if req.payload.get::<RequestedTransport>().is_none() {
        return reject(req, ErrorType::ServerError);
    }

    let Some((username, password)) = req.verify().await else {
        return reject(req, ErrorType::Unauthorized);
    };

    let lifetime = req.payload.get::<Lifetime>();

    let Some(port) = req.state.manager.allocate(req.id, lifetime) else {
        return reject(req, ErrorType::AllocationQuotaReached);
    };

    req.state.handler.on_allocated(req.id, username, port);

    {
        let mut message = MessageEncoder::extend(ALLOCATE_RESPONSE, req.payload, req.encode_buffer);
        message.append::<XorRelayedAddress>(SocketAddr::new(req.state.interface.ip(), port));
        message.append::<XorMappedAddress>(req.id.source());
        message.append::<Lifetime>(lifetime.unwrap_or(DEFAULT_SESSION_LIFETIME as u32));
        message.append::<Software>(&req.state.software);
        message.flush(Some(&password)).ok()?;
    }

    Some(Response {
        target: Target::default(),
        method: Some(ALLOCATE_RESPONSE),
        bytes: req.encode_buffer,
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
async fn channel_bind<'a, T>(req: Request<'_, 'a, T, Message<'_>>) -> Option<Response<'a>>
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

    let Some((username, password)) = req.verify().await else {
        return reject(req, ErrorType::Unauthorized);
    };

    if !req
        .state
        .manager
        .bind_channel(req.id, &req.state.endpoint, peer.port(), number)
    {
        return reject(req, ErrorType::Forbidden);
    }

    req.state.handler.on_channel_bind(req.id, username, number);

    {
        MessageEncoder::extend(CHANNEL_BIND_RESPONSE, req.payload, req.encode_buffer)
            .flush(Some(&password))
            .ok()?;
    }

    Some(Response {
        target: Target::default(),
        method: Some(CHANNEL_BIND_RESPONSE),
        bytes: req.encode_buffer,
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
async fn create_permission<'a, T>(req: Request<'_, 'a, T, Message<'_>>) -> Option<Response<'a>>
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

    if !req
        .state
        .manager
        .create_permission(req.id, &req.state.endpoint, &ports)
    {
        return reject(req, ErrorType::Forbidden);
    }

    req.state
        .handler
        .on_create_permission(req.id, username, &ports);

    {
        MessageEncoder::extend(CREATE_PERMISSION_RESPONSE, req.payload, req.encode_buffer)
            .flush(Some(&password))
            .ok()?;
    }

    Some(Response {
        method: Some(CREATE_PERMISSION_RESPONSE),
        target: Target::default(),
        bytes: req.encode_buffer,
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
///
/// # Internal Relay-to-Relay Forwarding
///
/// When two clients are both connected to the same TURN server and communicate
/// via their relay addresses, a special internal forwarding path is used.
/// Instead of looking up the permission table (which is keyed by the sender's
/// identifier), the server detects that the peer address is one of its own
/// relay ports and directly forwards the data to the client who owns that
/// relay port. This enables relay-to-relay communication for clients in
/// isolated networks that can only reach each other through the TURN server.
#[rustfmt::skip]
fn indication<'a, T>(req: Request<'_, 'a, T, Message<'_>>) -> Option<Response<'a>>
where
    T: ServiceHandler,
{
    let peer = req.payload.get::<XorPeerAddress>()?;
    let data = req.payload.get::<Data>()?;

    if let Some(Session::Authenticated { allocate_port, .. }) =
        req.state.manager.get_session(req.id).get_ref() && let Some(local_port) = *allocate_port
    {
        // Check if peer address is this server's own relay address (internal relay-to-relay)
        let is_internal_relay = req.state.interfaces.iter().any(|iface| iface.ip() == peer.ip())
            && req.state.manager.is_internal_relay_port(peer.port());

        let relay = if is_internal_relay {
            // Internal relay-to-relay: peer is another client on this same TURN server
            // This also validates that sender has permission to send to the target port
            match req.state.manager.get_internal_relay_endpoint(req.id, peer.port()) {
                Some(r) => {
                    log::debug!(
                        "TURN indication: internal relay-to-relay, peer relay port {} -> target {:?}",
                        peer.port(),
                        r.source()
                    );
                    r
                }
                None => {
                    log::warn!(
                        "TURN indication: internal relay port {} not found or no permission, \
                        src={:?}, peer={:?}",
                        peer.port(),
                        req.id.source(),
                        peer
                    );
                    return None;
                }
            }
        } else {
            // Normal case: use permission-based routing
            match req.state.manager.get_relay_address(req.id, peer.port()) {
                Some(r) => r,
                None => {
                    log::warn!(
                        "TURN indication: relay address not found for peer port {}, \
                        src={:?}, peer={:?}",
                        peer.port(),
                        req.id.source(),
                        peer
                    );
                    return None;
                }
            }
        };

        // Debug log: track endpoint for forwarding
        let target_endpoint = if req.state.endpoint != relay.endpoint() {
            Some(relay.endpoint())
        } else {
            None
        };
        
        log::debug!(
            "TURN indication: forwarding {} bytes, src={:?} -> peer={:?}, \
            relay.source={:?}, relay.endpoint={:?}, target_endpoint={:?}",
            data.len(),
            req.id.source(),
            peer,
            relay.source(),
            relay.endpoint(),
            target_endpoint
        );

        {
            let mut message = MessageEncoder::extend(DATA_INDICATION, req.payload, req.encode_buffer);
            message.append::<XorPeerAddress>(SocketAddr::new(req.state.interface.ip(), local_port));
            message.append::<Data>(data);
            message.flush(None).ok()?;
        }

        return Some(Response {
            method: Some(DATA_INDICATION),
            bytes: req.encode_buffer,
            target: Target {
                relay: Some(relay.source()),
                endpoint: target_endpoint,
            },
        });
    }

    log::debug!(
        "TURN indication: session not found or not authenticated, src={:?}",
        req.id.source()
    );
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
async fn refresh<'a, T>(req: Request<'_, 'a, T, Message<'_>>) -> Option<Response<'a>>
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
    if !req.state.manager.refresh(req.id, lifetime) {
        return reject(req, ErrorType::AllocationMismatch);
    }

    req.state.handler.on_refresh(req.id, username, lifetime);

    {
        let mut message = MessageEncoder::extend(REFRESH_RESPONSE, req.payload, req.encode_buffer);
        message.append::<Lifetime>(lifetime);
        message.flush(Some(&password)).ok()?;
    }

    Some(Response {
        target: Target::default(),
        method: Some(REFRESH_RESPONSE),
        bytes: req.encode_buffer,
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
fn channel_data<'a, T>(
    bytes: &'a [u8],
    req: Request<'_, 'a, T, ChannelData<'_>>,
) -> Option<Response<'a>>
where
    T: ServiceHandler,
{
    let relay = req
        .state
        .manager
        .get_channel_relay_address(req.id, req.payload.number())?;

    Some(Response {
        bytes,
        target: Target {
            relay: Some(relay.source()),
            endpoint: if req.state.endpoint != relay.endpoint() {
                Some(relay.endpoint())
            } else {
                None
            },
        },
        method: None,
    })
}
