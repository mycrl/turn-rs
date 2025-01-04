use crate::StunError;

use std::{
    fmt::Debug,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use bytes::{BufMut, BytesMut};
use num_enum::TryFromPrimitive;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
pub enum Transport {
    TCP = 0x06000000,
    UDP = 0x11000000,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpFamily {
    V4 = 0x01,
    V6 = 0x02,
}

impl TryFrom<u8> for IpFamily {
    type Error = StunError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0x01 => Self::V4,
            0x02 => Self::V6,
            _ => return Err(StunError::InvalidInput),
        })
    }
}

/// [RFC3489]: https://datatracker.ietf.org/doc/html/rfc3489
///
/// The Address attribute indicates a reflexive transport address
/// of the client.  It consists of an 8-bit address family and a 16-bit
/// port, followed by a fixed-length value representing the IP address.
/// If the address family is IPv4, the address MUST be 32 bits.  If the
/// address family is IPv6, the address MUST be 128 bits.  All fields
/// must be in network byte order.
///
/// The format of the MAPPED-ADDRESS attribute is:
///
/// ```text
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |0 0 0 0 0 0 0 0|    Family     |           Port                |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// |                 Address (32 bits or 128 bits)                 |
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
///
/// Figure 5: Format of MAPPED-ADDRESS Attribute
///
/// The address family can take on the following values:
///
/// * 0x01:IPv4
/// * 0x02:IPv6
///
/// The first 8 bits of the MAPPED-ADDRESS MUST be set to 0 and MUST be
/// ignored by receivers.  These bits are present for aligning parameters
/// on natural 32-bit boundaries.
///
/// This attribute is used only by servers for achieving backwards
/// compatibility with [RFC3489] clients.
///
/// The XOR-MAPPED-ADDRESS attribute is identical to the MAPPED-ADDRESS
/// attribute, except that the reflexive transport address is obfuscated
/// through the XOR function.
///
/// The format of the XOR-MAPPED-ADDRESS is:
///
/// ```text
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |0 0 0 0 0 0 0 0|    Family     |         X-Port                |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                X-Address (Variable)
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///
///          Figure 6: Format of XOR-MAPPED-ADDRESS Attribute
/// ```
///
/// The Family field represents the IP address family and is encoded
/// identically to the Family field in MAPPED-ADDRESS.
///
/// X-Port is computed by XOR'ing the mapped port with the most
/// significant 16 bits of the magic cookie.  If the IP address family is
/// IPv4, X-Address is computed by XOR'ing the mapped IP address with the
/// magic cookie.  If the IP address family is IPv6, X-Address is
/// computed by XOR'ing the mapped IP address with the concatenation of
/// the magic cookie and the 96-bit transaction ID.  In all cases, the
/// XOR operation works on its inputs in network byte order (that is, the
/// order they will be encoded in the message).
///
/// The rules for encoding and processing the first 8 bits of the
/// attribute's value, the rules for handling multiple occurrences of the
/// attribute, and the rules for processing address families are the same
/// as for MAPPED-ADDRESS.
///
/// Note: XOR-MAPPED-ADDRESS and MAPPED-ADDRESS differ only in their
/// encoding of the transport address.  The former encodes the transport
/// address by XOR'ing it with the magic cookie.  The latter encodes it
/// directly in binary.  [RFC3489] originally specified only MAPPED-
/// ADDRESS.  However, deployment experience found that some NATs rewrite
/// the 32-bit binary payloads containing the NAT's public IP address,
/// such as STUN's MAPPED-ADDRESS attribute, in the well-meaning but
/// misguided attempt to provide a generic Application Layer Gateway
/// (ALG) function.  Such behavior interferes with the operation of STUN
/// and also causes failure of STUN's message-integrity checking.
pub struct Addr;

impl Addr {
    /// encoder SocketAddr as Bytes.
    ///
    /// # Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use mycrl_stun::attribute::*;
    ///
    /// let xor_addr_bytes: [u8; 8] =
    ///     [0x00, 0x01, 0xfc, 0xbe, 0xe1, 0xba, 0xa4, 0x29];
    ///
    /// let addr_bytes: [u8; 8] = [0x00, 0x01, 0xdd, 0xac, 0xc0, 0xa8, 0x00, 0x6b];
    ///
    /// let token: [u8; 12] = [
    ///     0x6c, 0x46, 0x62, 0x54, 0x75, 0x4b, 0x44, 0x51, 0x46, 0x48, 0x4c, 0x71,
    /// ];
    ///
    /// let source = "192.168.0.107:56748".parse().unwrap();
    ///
    /// let mut buffer = BytesMut::with_capacity(1280);
    /// Addr::encode(&source, &token, &mut buffer, true);
    /// assert_eq!(&xor_addr_bytes, &buffer[..]);
    ///
    /// let mut buffer = BytesMut::with_capacity(1280);
    /// Addr::encode(&source, &token, &mut buffer, false);
    /// assert_eq!(&addr_bytes, &buffer[..]);
    /// ```
    pub fn encode(addr: &SocketAddr, token: &[u8], bytes: &mut BytesMut, is_xor: bool) {
        bytes.put_u8(0);
        let xor_addr = if is_xor { xor(addr, token) } else { *addr };

        bytes.put_u8(if xor_addr.is_ipv4() {
            IpFamily::V4
        } else {
            IpFamily::V6
        } as u8);

        bytes.put_u16(xor_addr.port());
        if let IpAddr::V4(ip) = xor_addr.ip() {
            bytes.put(&ip.octets()[..]);
        }

        if let IpAddr::V6(ip) = xor_addr.ip() {
            bytes.put(&ip.octets()[..]);
        }
    }

    /// decoder Bytes as SocketAddr.
    ///
    /// # Test
    ///
    /// ```
    /// use mycrl_stun::attribute::*;
    ///
    /// let xor_addr_bytes: [u8; 8] =
    ///     [0x00, 0x01, 0xfc, 0xbe, 0xe1, 0xba, 0xa4, 0x29];
    ///
    /// let addr_bytes: [u8; 8] = [0x00, 0x01, 0xdd, 0xac, 0xc0, 0xa8, 0x00, 0x6b];
    ///
    /// let token: [u8; 12] = [
    ///     0x6c, 0x46, 0x62, 0x54, 0x75, 0x4b, 0x44, 0x51, 0x46, 0x48, 0x4c, 0x71,
    /// ];
    ///
    /// let source = "192.168.0.107:56748".parse().unwrap();
    ///
    /// let addr = Addr::decode(&xor_addr_bytes, &token, true).unwrap();
    /// assert_eq!(addr, source);
    ///
    /// let addr = Addr::decode(&addr_bytes, &token, false).unwrap();
    /// assert_eq!(addr, source);
    /// ```
    pub fn decode(packet: &[u8], token: &[u8], is_xor: bool) -> Result<SocketAddr, StunError> {
        if packet.len() < 4 {
            return Err(StunError::InvalidInput);
        }

        let port = u16::from_be_bytes([packet[2], packet[3]]);
        let ip_addr = match IpFamily::try_from(packet[1])? {
            IpFamily::V4 => from_bytes_v4(packet)?,
            IpFamily::V6 => from_bytes_v6(packet)?,
        };

        let dyn_addr = SocketAddr::new(ip_addr, port);
        Ok(if is_xor {
            xor(&dyn_addr, token)
        } else {
            dyn_addr
        })
    }
}

/// # Test
///
/// ```
/// use std::net::IpAddr;
/// use mycrl_stun::attribute::*;
///
/// let bytes: [u8; 8] = [0x00, 0x01, 0xdd, 0xac, 0xc0, 0xa8, 0x00, 0x6b];
///
/// let source: IpAddr = "192.168.0.107".parse().unwrap();
///
/// let addr = from_bytes_v4(&bytes).unwrap();
/// assert_eq!(addr, source);
/// ```
pub fn from_bytes_v4(packet: &[u8]) -> Result<IpAddr, StunError> {
    if packet.len() != 8 {
        return Err(StunError::InvalidInput);
    }

    let bytes: [u8; 4] = packet[4..8].try_into()?;
    Ok(IpAddr::V4(bytes.into()))
}

/// # Test
///
/// ```
/// use std::net::IpAddr;
/// use mycrl_stun::attribute::*;
///
/// let bytes: [u8; 20] = [
///     0x00, 0x01, 0xdd, 0xac, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
///     0x00, 0x00, 0xFF, 0xFF, 0xC0, 0x0A, 0x2F, 0x0F,
/// ];
///
/// let source: IpAddr = "::ffff:192.10.47.15".parse().unwrap();
///
/// let addr = from_bytes_v6(&bytes).unwrap();
/// assert_eq!(addr, source);
/// ```
pub fn from_bytes_v6(packet: &[u8]) -> Result<IpAddr, StunError> {
    if packet.len() != 20 {
        return Err(StunError::InvalidInput);
    }

    let bytes: [u8; 16] = packet[4..20].try_into()?;
    Ok(IpAddr::V6(bytes.into()))
}

/// # Test
///
/// ```
/// use std::net::SocketAddr;
/// use mycrl_stun::attribute::*;
///
/// let source: SocketAddr = "192.168.0.107:1".parse().unwrap();
///
/// let res: SocketAddr = "225.186.164.41:8467".parse().unwrap();
///
/// let token: [u8; 12] = [
///     0x6c, 0x46, 0x62, 0x54, 0x75, 0x4b, 0x44, 0x51, 0x46, 0x48, 0x4c, 0x71,
/// ];
///
/// let addr = xor(&source, &token);
/// assert_eq!(addr, res);
/// ```
pub fn xor(addr: &SocketAddr, token: &[u8]) -> SocketAddr {
    let port = addr.port() ^ (0x2112A442 >> 16) as u16;
    let ip_addr = match addr.ip() {
        IpAddr::V4(x) => xor_v4(x),
        IpAddr::V6(x) => xor_v6(x, token),
    };

    SocketAddr::new(ip_addr, port)
}

/// # Test
///
/// ```
/// use std::net::{IpAddr, Ipv4Addr};
/// use mycrl_stun::attribute::*;
///
/// let source: Ipv4Addr = "192.168.0.107".parse().unwrap();
///
/// let xor: IpAddr = "225.186.164.41".parse().unwrap();
///
/// let addr = xor_v4(source);
/// assert_eq!(addr, xor);
/// ```
pub fn xor_v4(addr: Ipv4Addr) -> IpAddr {
    let mut octets = addr.octets();
    for (i, b) in octets.iter_mut().enumerate() {
        *b ^= (0x2112A442 >> (24 - i * 8)) as u8;
    }

    IpAddr::V4(From::from(octets))
}

/// # Test
///
/// ```
/// use std::net::{IpAddr, Ipv6Addr};
/// use mycrl_stun::attribute::*;
///
/// let source: Ipv6Addr = "::ffff:192.10.47.15".parse().unwrap();
///
/// let xor: IpAddr =
///     "2112:a442:6c46:6254:754b:bbae:8642:637e".parse().unwrap();
///
/// let token: [u8; 12] = [
///     0x6c, 0x46, 0x62, 0x54, 0x75, 0x4b, 0x44, 0x51, 0x46, 0x48, 0x4c, 0x71,
/// ];
///
/// let addr = xor_v6(source, &token);
/// assert_eq!(addr, xor);
/// ```
pub fn xor_v6(addr: Ipv6Addr, token: &[u8]) -> IpAddr {
    let mut octets = addr.octets();
    for (i, b) in octets.iter_mut().enumerate().take(4) {
        *b ^= (0x2112A442 >> (24 - i * 8)) as u8;
    }

    for (i, b) in octets.iter_mut().enumerate().take(16).skip(4) {
        *b ^= token[i - 4];
    }

    IpAddr::V6(From::from(octets))
}

/// STUN Attributes Registry
///
/// [RFC8126]: https://datatracker.ietf.org/doc/html/rfc8126
/// [RFC5389]: https://datatracker.ietf.org/doc/html/rfc5389
/// [RFC8489]: https://datatracker.ietf.org/doc/html/rfc8489
///
/// A STUN attribute type is a hex number in the range 0x0000-0xFFFF.
/// STUN attribute types in the range 0x0000-0x7FFF are considered
/// comprehension-required; STUN attribute types in the range
/// 0x8000-0xFFFF are considered comprehension-optional.  A STUN agent
/// handles unknown comprehension-required and comprehension-optional
/// attributes differently.
///
/// STUN attribute types in the first half of the comprehension-required
/// range (0x0000-0x3FFF) and in the first half of the comprehension-
/// optional range (0x8000-0xBFFF) are assigned by IETF Review [RFC8126].
/// STUN attribute types in the second half of the comprehension-required
/// range (0x4000-0x7FFF) and in the second half of the comprehension-
/// optional range (0xC000-0xFFFF) are assigned by Expert Review
/// [RFC8126].  The responsibility of the expert is to verify that the
/// selected codepoint(s) are not in use and that the request is not for
/// an abnormally large number of codepoints.  Technical review of the
/// extension itself is outside the scope of the designated expert
/// responsibility.
///
/// IANA has updated the names for attributes 0x0002, 0x0004, 0x0005,
/// 0x0007, and 0x000B as well as updated the reference from [RFC5389] to
/// [RFC8489] for each the following STUN methods.
///
/// In addition, [RFC5389] introduced a mistake in the name of attribute
/// 0x0003; [RFC5389] called it CHANGE-ADDRESS when it was actually
/// previously called CHANGE-REQUEST.  Thus, IANA has updated the
/// description for 0x0003 to read "Reserved; was CHANGE-REQUEST prior to
/// [RFC5389]".
///
/// Comprehension-required range (0x0000-0x7FFF):
/// 0x0000: Reserved
/// 0x0001: MAPPED-ADDRESS
/// 0x0002: Reserved; was RESPONSE-ADDRESS prior to [RFC5389]
/// 0x0003: Reserved; was CHANGE-REQUEST prior to [RFC5389]
/// 0x0004: Reserved; was SOURCE-ADDRESS prior to [RFC5389]
/// 0x0005: Reserved; was CHANGED-ADDRESS prior to [RFC5389]
/// 0x0006: USERNAME
/// 0x0007: Reserved; was PASSWORD prior to [RFC5389]
/// 0x0008: MESSAGE-INTEGRITY
/// 0x0009: ERROR-CODE
/// 0x000A: UNKNOWN-ATTRIBUTES
/// 0x000B: Reserved; was REFLECTED-FROM prior to [RFC5389]
/// 0x0014: REALM
/// 0x0015: NONCE
/// 0x0020: XOR-MAPPED-ADDRESS
///
/// Comprehension-optional range (0x8000-0xFFFF)
/// 0x8022: SOFTWARE
///  0x8023: ALTERNATE-SERVER
/// 0x8028: FINGERPRINT
///
/// IANA has added the following attribute to the "STUN Attributes"
/// registry:
///
/// Comprehension-required range (0x0000-0x7FFF):
/// 0x001C: MESSAGE-INTEGRITY-SHA256
/// 0x001D: PASSWORD-ALGORITHM
///  0x001E: USERHASH
///
/// Comprehension-optional range (0x8000-0xFFFF)
/// 0x8002: PASSWORD-ALGORITHMS
/// 0x8003: ALTERNATE-DOMAIN
#[repr(u16)]
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug, TryFromPrimitive)]
pub enum AttrKind {
    #[default]
    Unknown = 0x0000,
    MappedAddress = 0x0001,
    UserName = 0x0006,
    MessageIntegrity = 0x0008,
    ErrorCode = 0x0009,
    ChannelNumber = 0x000C,
    Lifetime = 0x000D,
    XorPeerAddress = 0x0012,
    Data = 0x0013,
    Realm = 0x0014,
    Nonce = 0x0015,
    XorRelayedAddress = 0x0016,
    RequestedAddressFamily = 0x0017,
    EvenPort = 0x0018,
    ReqeestedTransport = 0x0019,
    DontFragment = 0x001A,
    XorMappedAddress = 0x0020,
    ReservationToken = 0x0022,
    Priority = 0x0024,
    UseCandidate = 0x0025,
    AdditionalAddressFamily = 0x8000,
    AddressErrorCode = 0x8001,
    Icmp = 0x8004,
    Software = 0x8022,
    Fingerprint = 0x8028,
    IceControlled = 0x8029,
    IceControlling = 0x802A,
    ResponseOrigin = 0x802B,
}

/// dyn stun/turn message attribute.
pub trait Attribute<'a> {
    type Error: Debug;

    /// current attribute inner type.
    type Item;

    /// current attribute type.
    const KIND: AttrKind;

    /// write the current attribute to the bytesfer.
    #[allow(unused_variables)]
    fn encode(value: Self::Item, bytes: &mut BytesMut, token: &'a [u8]) {}

    /// convert bytesfer to current attribute.
    fn decode(bytes: &'a [u8], token: &'a [u8]) -> Result<Self::Item, Self::Error>;
}

/// [RFC8265]: https://datatracker.ietf.org/doc/html/rfc8265
/// [RFC5389]: https://datatracker.ietf.org/doc/html/rfc5389
/// [RFC3629]: https://datatracker.ietf.org/doc/html/rfc3629
///
/// The USERNAME attribute is used for message integrity.  It identifies
/// the username and password combination used in the message-integrity
/// check.
///
/// The value of USERNAME is a variable-length value containing the
/// authentication username.  It MUST contain a UTF-8-encoded [RFC3629]
/// sequence of fewer than 509 bytes and MUST have been processed using
/// the OpaqueString profile [RFC8265].  A compliant implementation MUST
/// be able to parse a UTF-8-encoded sequence of 763 or fewer octets to
/// be compatible with [RFC5389].
pub struct UserName;

impl<'a> Attribute<'a> for UserName {
    type Error = StunError;
    type Item = &'a str;

    const KIND: AttrKind = AttrKind::UserName;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put(value.as_bytes());
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(std::str::from_utf8(bytes)?)
    }
}

/// The DATA attribute is present in all Send and Data indications.  The
/// value portion of this attribute is variable length and consists of
/// the application data (that is, the data that would immediately follow
/// the UDP header if the data was been sent directly between the client
/// and the peer).  If the length of this attribute is not a multiple of
/// 4, then padding must be added after this attribute.
pub struct Data;

impl<'a> Attribute<'a> for Data {
    type Error = StunError;
    type Item = &'a [u8];

    const KIND: AttrKind = AttrKind::Data;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put(value);
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(bytes)
    }
}

/// [RFC3629]: https://datatracker.ietf.org/doc/html/rfc3629
/// [RFC3261]: https://datatracker.ietf.org/doc/html/rfc3261
/// [RFC8265]: https://datatracker.ietf.org/doc/html/rfc8265
///
/// The REALM attribute may be present in requests and responses.  It
/// contains text that meets the grammar for "realm-value" as described
/// in [RFC3261] but without the double quotes and their surrounding
/// whitespace.  That is, it is an unquoted realm-value (and is therefore
/// a sequence of qdtext or quoted-pair).  It MUST be a UTF-8-encoded
/// [RFC3629] sequence of fewer than 128 characters (which can be as long
/// as 509 bytes when encoding them and as long as 763 bytes when
/// decoding them) and MUST have been processed using the OpaqueString
/// profile [RFC8265].
///
/// Presence of the REALM attribute in a request indicates that long-term
/// credentials are being used for authentication.  Presence in certain
/// error responses indicates that the server wishes the client to use a
/// long-term credential in that realm for authentication.
pub struct Realm;

impl<'a> Attribute<'a> for Realm {
    type Error = StunError;
    type Item = &'a str;

    const KIND: AttrKind = AttrKind::Realm;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put(value.as_bytes());
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(std::str::from_utf8(bytes)?)
    }
}

/// [RFC3261]: https://datatracker.ietf.org/doc/html/rfc3261
/// [RFC7616]: https://datatracker.ietf.org/doc/html/rfc7616
///
/// The NONCE attribute may be present in requests and responses.  It
/// contains a sequence of qdtext or quoted-pair, which are defined in
/// [RFC3261].  Note that this means that the NONCE attribute will not
/// contain the actual surrounding quote characters.  The NONCE attribute
/// MUST be fewer than 128 characters (which can be as long as 509 bytes
/// when encoding them and a long as 763 bytes when decoding them).  See
/// Section 5.4 of [RFC7616] for guidance on selection of nonce values in
/// a server.
pub struct Nonce;

impl<'a> Attribute<'a> for Nonce {
    type Error = StunError;
    type Item = &'a str;

    const KIND: AttrKind = AttrKind::Nonce;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put(value.as_bytes());
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(std::str::from_utf8(bytes)?)
    }
}

/// [RFC3629]: https://datatracker.ietf.org/doc/html/rfc3629
///
/// The SOFTWARE attribute contains a textual description of the software
/// being used by the agent sending the message.  It is used by clients
/// and servers.  Its value SHOULD include manufacturer and version
/// number.  The attribute has no impact on operation of the protocol and
/// serves only as a tool for diagnostic and debugging purposes.  The
/// value of SOFTWARE is variable length.  It MUST be a UTF-8-encoded
/// [RFC3629] sequence of fewer than 128 characters (which can be as long
/// as 509 when encoding them and as long as 763 bytes when decoding
/// them).
pub struct Software;

impl<'a> Attribute<'a> for Software {
    type Error = StunError;
    type Item = &'a str;

    const KIND: AttrKind = AttrKind::Software;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put(value.as_bytes());
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(std::str::from_utf8(bytes)?)
    }
}

/// [RFC2104]: https://datatracker.ietf.org/doc/html/rfc2104
/// [RFC5769]: https://datatracker.ietf.org/doc/html/rfc5769
///
/// The MESSAGE-INTEGRITY attribute contains an HMAC-SHA1 [RFC2104] of
/// the STUN message.  The MESSAGE-INTEGRITY attribute can be present in
/// any STUN message type.  Since it uses the SHA-1 hash, the HMAC will
/// be 20 bytes.
///
/// The key for the HMAC depends on which credential mechanism is in use.
/// Section 9.1.1 defines the key for the short-term credential
/// mechanism, and Section 9.2.2 defines the key for the long-term
/// credential mechanism.  Other credential mechanisms MUST define the
/// key that is used for the HMAC.
///
/// The text used as input to HMAC is the STUN message, up to and
/// including the attribute preceding the MESSAGE-INTEGRITY attribute.
/// The Length field of the STUN message header is adjusted to point to
/// the end of the MESSAGE-INTEGRITY attribute.  The value of the
/// MESSAGE-INTEGRITY attribute is set to a dummy value.
///
/// Once the computation is performed, the value of the MESSAGE-INTEGRITY
/// attribute is filled in, and the value of the length in the STUN
/// header is set to its correct value -- the length of the entire
/// message.  Similarly, when validating the MESSAGE-INTEGRITY, the
/// Length field in the STUN header must be adjusted to point to the end
/// of the MESSAGE-INTEGRITY attribute prior to calculating the HMAC over
/// the STUN message, up to and including the attribute preceding the
/// MESSAGE-INTEGRITY attribute.  Such adjustment is necessary when
/// attributes, such as FINGERPRINT and MESSAGE-INTEGRITY-SHA256, appear
/// after MESSAGE-INTEGRITY.  See also [RFC5769] for examples of such
/// calculations.
pub struct MessageIntegrity;

impl<'a> Attribute<'a> for MessageIntegrity {
    type Error = StunError;
    type Item = &'a [u8];

    const KIND: AttrKind = AttrKind::MessageIntegrity;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put(value);
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(bytes)
    }
}

/// [RFC5389]: https://datatracker.ietf.org/doc/html/rfc5389
///
/// The XOR-PEER-ADDRESS specifies the address and port of the peer as
/// seen from the TURN server.  (For example, the peer's server-reflexive
/// transport address if the peer is behind a NAT.)  It is encoded in the
/// same way as XOR-MAPPED-ADDRESS [RFC5389].
pub struct XorPeerAddress;

impl<'a> Attribute<'a> for XorPeerAddress {
    type Error = StunError;
    type Item = SocketAddr;

    const KIND: AttrKind = AttrKind::XorPeerAddress;

    fn encode(value: Self::Item, bytes: &mut BytesMut, token: &'a [u8]) {
        Addr::encode(&value, token, bytes, true)
    }

    fn decode(bytes: &'a [u8], token: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Addr::decode(bytes, token, true)
    }
}

/// [RFC5389]: https://datatracker.ietf.org/doc/html/rfc5389
///
/// The XOR-RELAYED-ADDRESS is present in Allocate responses.  It
/// specifies the address and port that the server allocated to the
/// client.  It is encoded in the same way as XOR-MAPPED-ADDRESS
/// [RFC5389].
pub struct XorRelayedAddress;

impl<'a> Attribute<'a> for XorRelayedAddress {
    type Error = StunError;
    type Item = SocketAddr;

    const KIND: AttrKind = AttrKind::XorRelayedAddress;

    fn encode(value: Self::Item, bytes: &mut BytesMut, token: &'a [u8]) {
        Addr::encode(&value, token, bytes, true)
    }

    fn decode(bytes: &'a [u8], token: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Addr::decode(bytes, token, true)
    }
}

/// [RFC3489]: https://datatracker.ietf.org/doc/html/rfc3489
///
/// The XOR-MAPPED-ADDRESS attribute is identical to the MAPPED-ADDRESS
/// attribute, except that the reflexive transport address is obfuscated
/// through the XOR function.
///
/// The Family field represents the IP address family and is encoded
/// identically to the Family field in MAPPED-ADDRESS.
///
/// X-Port is computed by XOR'ing the mapped port with the most
/// significant 16 bits of the magic cookie.  If the IP address family is
/// IPv4, X-Address is computed by XOR'ing the mapped IP address with the
/// magic cookie.  If the IP address family is IPv6, X-Address is
/// computed by XOR'ing the mapped IP address with the concatenation of
/// the magic cookie and the 96-bit transaction ID.  In all cases, the
/// XOR operation works on its inputs in network byte order (that is, the
/// order they will be encoded in the message).
///
/// The rules for encoding and processing the first 8 bits of the
/// attribute's value, the rules for handling multiple occurrences of the
/// attribute, and the rules for processing address families are the same
/// as for MAPPED-ADDRESS.
///
/// Note: XOR-MAPPED-ADDRESS and MAPPED-ADDRESS differ only in their
/// encoding of the transport address.  The former encodes the transport
/// address by XOR'ing it with the magic cookie.  The latter encodes it
/// directly in binary.  [RFC3489] originally specified only MAPPED-
/// ADDRESS.  However, deployment experience found that some NATs rewrite
/// the 32-bit binary payloads containing the NAT's public IP address,
/// such as STUN's MAPPED-ADDRESS attribute, in the well-meaning but
/// misguided attempt to provide a generic Application Layer Gateway
/// (ALG) function.  Such behavior interferes with the operation of STUN
/// and also causes failure of STUN's message-integrity checking.
pub struct XorMappedAddress;

impl<'a> Attribute<'a> for XorMappedAddress {
    type Error = StunError;
    type Item = SocketAddr;

    const KIND: AttrKind = AttrKind::XorMappedAddress;

    fn encode(value: Self::Item, bytes: &mut BytesMut, token: &'a [u8]) {
        Addr::encode(&value, token, bytes, true)
    }

    fn decode(bytes: &'a [u8], token: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Addr::decode(bytes, token, true)
    }
}

/// [RFC3489]: https://datatracker.ietf.org/doc/html/rfc3489
///
/// The MAPPED-ADDRESS attribute indicates a reflexive transport address
/// of the client.  It consists of an 8-bit address family and a 16-bit
/// port, followed by a fixed-length value representing the IP address.
/// If the address family is IPv4, the address MUST be 32 bits.  If the
/// address family is IPv6, the address MUST be 128 bits.  All fields
/// must be in network byte order.
///
/// The address family can take on the following values:
///
/// 0x01:IPv4
/// 0x02:IPv6
///
/// The first 8 bits of the MAPPED-ADDRESS MUST be set to 0 and MUST be
/// ignored by receivers.  These bits are present for aligning parameters
/// on natural 32-bit boundaries.
///
/// This attribute is used only by servers for achieving backwards
/// compatibility with [RFC3489] clients.
pub struct MappedAddress;

impl<'a> Attribute<'a> for MappedAddress {
    type Error = StunError;
    type Item = SocketAddr;

    const KIND: AttrKind = AttrKind::MappedAddress;

    fn encode(value: Self::Item, bytes: &mut BytesMut, token: &'a [u8]) {
        Addr::encode(&value, token, bytes, false)
    }

    fn decode(bytes: &'a [u8], token: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Addr::decode(bytes, token, false)
    }
}

/// The RESPONSE-ORIGIN attribute is inserted by the server and indicates
/// the source IP address and port the response was sent from.  It is
/// useful for detecting double NAT configurations.  It is only present
/// in Binding Responses.
pub struct ResponseOrigin;

impl<'a> Attribute<'a> for ResponseOrigin {
    type Error = StunError;
    type Item = SocketAddr;

    const KIND: AttrKind = AttrKind::ResponseOrigin;

    fn encode(value: Self::Item, bytes: &mut BytesMut, token: &'a [u8]) {
        Addr::encode(&value, token, bytes, false)
    }

    fn decode(bytes: &'a [u8], token: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Addr::decode(bytes, token, false)
    }
}

/// The following error codes, along with their recommended reason
/// phrases, are defined:
///
/// 300  Try Alternate: The client should contact an alternate server for
///      this request.  This error response MUST only be sent if the
///      request included either a USERNAME or USERHASH attribute and a
///      valid MESSAGE-INTEGRITY or MESSAGE-INTEGRITY-SHA256 attribute;
///      otherwise, it MUST NOT be sent and error code 400 (Bad Request)
///      is suggested.  This error response MUST be protected with the
///      MESSAGE-INTEGRITY or MESSAGE-INTEGRITY-SHA256 attribute, and
///      receivers MUST validate the MESSAGE-INTEGRITY or MESSAGE-
///      INTEGRITY-SHA256 of this response before redirecting themselves
///      to an alternate server.
///      Note: Failure to generate and validate message integrity for a
///      300 response allows an on-path attacker to falsify a 300
///      response thus causing subsequent STUN messages to be sent to a
///      victim.
///      
/// 400  Bad Request: The request was malformed.  The client SHOULD NOT
///      retry the request without modification from the previous
///      attempt.  The server may not be able to generate a valid
///      MESSAGE-INTEGRITY or MESSAGE-INTEGRITY-SHA256 for this error, so
///      the client MUST NOT expect a valid MESSAGE-INTEGRITY or MESSAGE-
///      INTEGRITY-SHA256 attribute on this response.
///      
/// 401  Unauthenticated: The request did not contain the correct
///      credentials to proceed.  The client should retry the request
///      with proper credentials.
///      
/// 420  Unknown Attribute: The server received a STUN packet containing
///      a comprehension-required attribute that it did not understand.
///      The server MUST put this unknown attribute in the UNKNOWN-
///      ATTRIBUTE attribute of its error response.
///      
/// 438  Stale Nonce: The NONCE used by the client was no longer valid.
///      The client should retry, using the NONCE provided in the
///      response.
///      
/// 500  Server Error: The server has suffered a temporary error.  The
///      client should try again.
const fn errno(code: u16) -> u16 {
    ((code / 100) << 8) | (code % 100)
}

#[repr(u16)]
#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash, TryFromPrimitive)]
pub enum ErrorKind {
    TryAlternate = errno(300),
    BadRequest = errno(400),
    Unauthorized = errno(401),
    Forbidden = errno(403),
    UnknownAttribute = errno(420),
    AllocationMismatch = errno(437),
    StaleNonce = errno(438),
    AddressFamilyNotSupported = errno(440),
    WrongCredentials = errno(441),
    UnsupportedTransportAddress = errno(442),
    PeerAddressFamilyMismatch = errno(443),
    AllocationQuotaReached = errno(486),
    ServerError = errno(500),
    InsufficientCapacity = errno(508),
}

/// [RFC3629]: https://datatracker.ietf.org/doc/html/rfc3629
/// [RFC7231]: https://datatracker.ietf.org/doc/html/rfc7231
/// [RFC3261]: https://datatracker.ietf.org/doc/html/rfc3261
/// [RFC3629]: https://datatracker.ietf.org/doc/html/rfc3629
///
/// The ERROR-CODE attribute is used in error response messages.  It
/// contains a numeric error code value in the range of 300 to 699 plus a
/// textual reason phrase encoded in UTF-8 [RFC3629]; it is also
/// consistent in its code assignments and semantics with SIP [RFC3261]
/// and HTTP [RFC7231].  The reason phrase is meant for diagnostic
/// purposes and can be anything appropriate for the error code.
/// Recommended reason phrases for the defined error codes are included
/// in the IANA registry for error codes.  The reason phrase MUST be a
/// UTF-8-encoded [RFC3629] sequence of fewer than 128 characters (which
/// can be as long as 509 bytes when encoding them or 763 bytes when
/// decoding them).
///
/// ```text
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |           Reserved, should be 0         |Class|     Number    |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |      Reason Phrase (variable)                                ..
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///
///              Figure 7: Format of ERROR-CODE Attribute
/// ```
///
/// To facilitate processing, the class of the error code (the hundreds
/// digit) is encoded separately from the rest of the code, as shown in
/// Figure 7.
///
/// The Reserved bits SHOULD be 0 and are for alignment on 32-bit
/// boundaries.  Receivers MUST ignore these bits.  The Class represents
/// the hundreds digit of the error code.  The value MUST be between 3
/// and 6.  The Number represents the binary encoding of the error code
/// modulo 100, and its value MUST be between 0 and 99.
#[derive(Clone, Debug)]
pub struct Error<'a> {
    pub code: u16,
    pub message: &'a str,
}

impl From<ErrorKind> for Error<'_> {
    /// create error from error type.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mycrl_stun::attribute::*;
    ///
    /// Error::from(ErrorKind::TryAlternate);
    /// ```
    fn from(value: ErrorKind) -> Self {
        Self {
            code: value as u16,
            message: value.into(),
        }
    }
}

impl Error<'_> {
    /// encode the error type as bytes.
    ///
    /// # Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use mycrl_stun::attribute::*;
    ///
    /// let buffer = [
    ///     0x00u8, 0x00, 0x03, 0x00, 0x54, 0x72, 0x79, 0x20, 0x41, 0x6c, 0x74,
    ///     0x65, 0x72, 0x6e, 0x61, 0x74, 0x65,
    /// ];
    ///
    /// let mut buf = BytesMut::with_capacity(1280);
    /// let error = Error::from(ErrorKind::TryAlternate);
    /// error.encode(&mut buf);
    /// assert_eq!(&buf[..], &buffer);
    /// ```
    pub fn encode(self, bytes: &mut BytesMut) {
        bytes.put_u16(0x0000);
        bytes.put_u16(self.code);
        bytes.put(self.message.as_bytes());
    }
}

impl<'a> TryFrom<&'a [u8]> for Error<'a> {
    type Error = StunError;

    /// # Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use mycrl_stun::attribute::*;
    ///
    /// let buffer = [
    ///     0x00u8, 0x00, 0x03, 0x00, 0x54, 0x72, 0x79, 0x20, 0x41, 0x6c, 0x74,
    ///     0x65, 0x72, 0x6e, 0x61, 0x74, 0x65,
    /// ];
    ///
    /// let error = Error::try_from(&buffer[..]).unwrap();
    /// assert_eq!(error.code, ErrorKind::TryAlternate as u16);
    /// assert_eq!(error.message, "Try Alternate");
    /// ```
    fn try_from(packet: &'a [u8]) -> Result<Self, Self::Error> {
        if packet.len() < 4 {
            return Err(StunError::InvalidInput);
        }

        if u16::from_be_bytes(packet[..2].try_into()?) != 0x0000 {
            return Err(StunError::InvalidInput);
        }

        Ok(Self {
            code: u16::from_be_bytes(packet[2..4].try_into()?),
            message: std::str::from_utf8(&packet[4..])?,
        })
    }
}

impl From<ErrorKind> for &'static str {
    /// # Test
    ///
    /// ```
    /// use std::convert::Into;
    /// use mycrl_stun::attribute::*;
    ///
    /// let err: &'static str = ErrorKind::TryAlternate.into();
    /// assert_eq!(err, "Try Alternate");
    /// ```
    #[rustfmt::skip]
    fn from(val: ErrorKind) -> Self {
        match val {
            ErrorKind::TryAlternate => "Try Alternate",
            ErrorKind::BadRequest => "Bad Request",
            ErrorKind::Unauthorized => "Unauthorized",
            ErrorKind::Forbidden => "Forbidden",
            ErrorKind::UnknownAttribute => "Unknown Attribute",
            ErrorKind::AllocationMismatch => "Allocation Mismatch",
            ErrorKind::StaleNonce => "Stale Nonce",
            ErrorKind::AddressFamilyNotSupported => "Address Family not Supported",
            ErrorKind::WrongCredentials => "Wrong Credentials",
            ErrorKind::UnsupportedTransportAddress => "Unsupported Transport Address",
            ErrorKind::AllocationQuotaReached => "Allocation Quota Reached",
            ErrorKind::ServerError => "Server Error",
            ErrorKind::InsufficientCapacity => "Insufficient Capacity",
            ErrorKind::PeerAddressFamilyMismatch => "Peer Address Family Mismatch",
        }
    }
}

impl Eq for Error<'_> {}
impl PartialEq for Error<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code
    }
}

/// [RFC7231]: https://datatracker.ietf.org/doc/html/rfc7231
/// [RFC3261]: https://datatracker.ietf.org/doc/html/rfc3261
/// [RFC3629]: https://datatracker.ietf.org/doc/html/rfc3629
///
/// The ERROR-CODE attribute is used in error response messages.  It
/// contains a numeric error code value in the range of 300 to 699 plus a
/// textual reason phrase encoded in UTF-8 [RFC3629]; it is also
/// consistent in its code assignments and semantics with SIP [RFC3261]
/// and HTTP [RFC7231].  The reason phrase is meant for diagnostic
/// purposes and can be anything appropriate for the error code.
/// Recommended reason phrases for the defined error codes are included
/// in the IANA registry for error codes.  The reason phrase MUST be a
/// UTF-8-encoded [RFC3629] sequence of fewer than 128 characters (which
/// can be as long as 509 bytes when encoding them or 763 bytes when
/// decoding them).
pub struct ErrorCode;

impl<'a> Attribute<'a> for ErrorCode {
    type Error = StunError;
    type Item = Error<'a>;

    const KIND: AttrKind = AttrKind::ErrorCode;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        value.encode(bytes)
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Error::try_from(bytes)
    }
}

/// The LIFETIME attribute represents the duration for which the server
/// will maintain an allocation in the absence of a refresh.  The value
/// portion of this attribute is 4-bytes long and consists of a 32-bit
/// unsigned integral value representing the number of seconds remaining
/// until expiration.
pub struct Lifetime;

impl<'a> Attribute<'a> for Lifetime {
    type Error = StunError;
    type Item = u32;

    const KIND: AttrKind = AttrKind::Lifetime;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put_u32(value)
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u32::from_be_bytes(bytes.try_into()?))
    }
}

/// This attribute is used by the client to request a specific transport
/// protocol for the allocated transport address.
///
/// The Protocol field specifies the desired protocol.  The codepoints
/// used in this field are taken from those allowed in the Protocol field
/// in the IPv4 header and the NextHeader field in the IPv6 header
/// [Protocol-Numbers].  This specification only allows the use of
/// codepoint 17 (User Datagram Protocol).
///
/// The RFFU field MUST be set to zero on transmission and MUST be
/// ignored on reception.  It is reserved for future uses.
pub struct ReqeestedTransport;

impl<'a> Attribute<'a> for ReqeestedTransport {
    type Error = StunError;
    type Item = Transport;

    const KIND: AttrKind = AttrKind::ReqeestedTransport;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put_u32(value as u32)
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        let value = u32::from_be_bytes(bytes.try_into()?);
        Transport::try_from(value).map_err(|_| StunError::InvalidInput)
    }
}

/// [RFC1952]: https://datatracker.ietf.org/doc/html/rfc1952
///
/// The FINGERPRINT attribute MAY be present in all STUN messages.
///
/// The value of the attribute is computed as the CRC-32 of the STUN
/// message up to (but excluding) the FINGERPRINT attribute itself,
/// XOR'ed with the 32-bit value 0x5354554e.  (The XOR operation ensures
/// that the FINGERPRINT test will not report a false positive on a
/// packet containing a CRC-32 generated by an application protocol.)
///
/// The 32-bit CRC is the one defined in ITU V.42, which has a generator
/// polynomial of x^32 + x^26 + x^23 + x^22 + x^16 + x^12 + x^11 + x^10 + x^8 +
/// x^7 + x^5 + x^4 + x^2 + x + 1.  See the sample code for the CRC-32 in
/// Section 8 of [RFC1952].
///
/// When present, the FINGERPRINT attribute MUST be the last attribute in
/// the message and thus will appear after MESSAGE-INTEGRITY and MESSAGE-
/// INTEGRITY-SHA256.
///
/// The FINGERPRINT attribute can aid in distinguishing STUN packets from
/// packets of other protocols.  See Section 7.
///
/// As with MESSAGE-INTEGRITY and MESSAGE-INTEGRITY-SHA256, the CRC used
/// in the FINGERPRINT attribute covers the Length field from the STUN
/// message header.  Therefore, prior to computation of the CRC, this
/// value must be correct and include the CRC attribute as part of the
/// message length.  When using the FINGERPRINT attribute in a message,
/// the attribute is first placed into the message with a dummy value;
/// then, the CRC is computed, and the value of the attribute is updated.
/// If the MESSAGE-INTEGRITY or MESSAGE-INTEGRITY-SHA256 attribute is
/// also present, then it must be present with the correct message-
/// integrity value before the CRC is computed, since the CRC is done
/// over the value of the MESSAGE-INTEGRITY and MESSAGE-INTEGRITY-SHA256
/// attributes as well.
pub struct Fingerprint;

impl<'a> Attribute<'a> for Fingerprint {
    type Error = StunError;
    type Item = u32;

    const KIND: AttrKind = AttrKind::Fingerprint;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put_u32(value)
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u32::from_be_bytes(bytes.try_into()?))
    }
}

/// The CHANNEL-NUMBER attribute contains the number of the channel.  The
/// value portion of this attribute is 4 bytes long and consists of a
/// 16-bit unsigned integer followed by a two-octet RFFU (Reserved For
/// Future Use) field, which MUST be set to 0 on transmission and MUST be
/// ignored on reception.
pub struct ChannelNumber;

impl<'a> Attribute<'a> for ChannelNumber {
    type Error = StunError;
    type Item = u16;

    const KIND: AttrKind = AttrKind::ChannelNumber;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put_u16(value)
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u16::from_be_bytes(bytes[..2].try_into()?))
    }
}

/// The ICE-CONTROLLING attribute is present in a Binding request.  The
/// attribute indicates that the client believes it is currently in the
/// controlling role.  The content of the attribute is a 64-bit unsigned
/// integer in network byte order, which contains a random number.  As
/// for the ICE-CONTROLLED attribute, the number is used for solving role
/// conflicts.  An agent MUST use the same number for all Binding
/// requests, for all streams, within an ICE session, unless it has
/// received a 487 response, in which case it MUST change the number.  
/// The agent MAY change the number when an ICE restart occurs.
pub struct IceControlling;

impl<'a> Attribute<'a> for IceControlling {
    type Error = StunError;
    type Item = u64;

    const KIND: AttrKind = AttrKind::IceControlling;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put_u64(value)
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u64::from_be_bytes(bytes.try_into()?))
    }
}

/// The USE-CANDIDATE attribute indicates that the candidate pair
/// resulting from this check will be used for transmission of data.  The
/// attribute has no content (the Length field of the attribute is zero);
/// it serves as a flag.  It has an attribute value of 0x0025..
pub struct UseCandidate;

impl<'a> Attribute<'a> for UseCandidate {
    type Error = StunError;
    type Item = ();

    const KIND: AttrKind = AttrKind::UseCandidate;

    fn encode(_: Self::Item, _: &mut BytesMut, _: &'a [u8]) {}

    fn decode(_: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(())
    }
}

/// The ICE-CONTROLLED attribute is present in a Binding request.  The
/// attribute indicates that the client believes it is currently in the
/// controlled role.  The content of the attribute is a 64-bit unsigned
/// integer in network byte order, which contains a random number.  The
/// number is used for solving role conflicts, when it is referred to as
/// the "tiebreaker value".  An ICE agent MUST use the same number for
/// all Binding requests, for all streams, within an ICE session, unless
/// it has received a 487 response, in which case it MUST change the
/// number. The agent MAY change the number when an ICE restart occurs.
pub struct IceControlled;

impl<'a> Attribute<'a> for IceControlled {
    type Error = StunError;
    type Item = u64;

    const KIND: AttrKind = AttrKind::IceControlled;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put_u64(value)
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u64::from_be_bytes(bytes.try_into()?))
    }
}

/// The PRIORITY attribute indicates the priority that is to be
/// associated with a peer-reflexive candidate, if one will be discovered
/// by this check.  It is a 32-bit unsigned integer and has an attribute
/// value of 0x0024.
pub struct Priority;

impl<'a> Attribute<'a> for Priority {
    type Error = StunError;
    type Item = u32;

    const KIND: AttrKind = AttrKind::Priority;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put_u32(value)
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u32::from_be_bytes(bytes.try_into()?))
    }
}

/// The RESERVATION-TOKEN attribute contains a token that uniquely identifies a
/// relayed transport address being held in reserve by the server. The server
/// includes this attribute in a success response to tell the client about the
/// token, and the client includes this attribute in a subsequent Allocate
/// request to request the server use that relayed transport address for the
/// allocation.
///
/// The attribute value is 8 bytes and contains the token value.
pub struct ReservationToken;

impl<'a> Attribute<'a> for ReservationToken {
    type Error = StunError;
    type Item = u64;

    const KIND: AttrKind = AttrKind::ReservationToken;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put_u64(value)
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u64::from_be_bytes(bytes.try_into()?))
    }
}

/// This attribute allows the client to request that the port in the relayed
/// transport address be even, and (optionally) that the server reserve the
/// next-higher port number.  The value portion of this attribute is 1 byte
/// long.
pub struct EvenPort;

impl<'a> Attribute<'a> for EvenPort {
    type Error = StunError;
    type Item = bool;

    const KIND: AttrKind = AttrKind::EvenPort;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put_u8(if value { 0b10000000 } else { 0b00000000 })
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(bytes[0] == 0b10000000)
    }
}

/// The REQUESTED-ADDRESS-FAMILY attribute is used by clients to request the
/// allocation of a specific address type from a server.  The following is the
/// format of the REQUESTED-ADDRESS-FAMILY attribute. Note that TURN attributes
/// are TLV (Type-Length-Value) encoded, with a 16-bit type, a 16-bit length,
/// and a variable-length value.
pub struct RequestedAddressFamily;

impl<'a> Attribute<'a> for RequestedAddressFamily {
    type Error = StunError;
    type Item = IpFamily;

    const KIND: AttrKind = AttrKind::RequestedAddressFamily;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put_u8(value as u8)
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        IpFamily::try_from(bytes[0])
    }
}

/// This attribute is used by clients to request the allocation of an IPv4 and
/// IPv6 address type from a server. It is encoded in the same way as the
/// REQUESTED-ADDRESS-FAMILY attribute; The ADDITIONAL-ADDRESS-FAMILY attribute
/// MAY be present in the Allocate request. The attribute value of 0x02 (IPv6
/// address) is the only valid value in Allocate request.
pub struct AdditionalAddressFamily;

impl<'a> Attribute<'a> for AdditionalAddressFamily {
    type Error = StunError;
    type Item = IpFamily;

    const KIND: AttrKind = AttrKind::AdditionalAddressFamily;

    fn encode(value: Self::Item, bytes: &mut BytesMut, _: &'a [u8]) {
        bytes.put_u8(value as u8)
    }

    fn decode(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        IpFamily::try_from(bytes[0])
    }
}

/// This attribute is used by the client to request that the server set the DF
/// (Don't Fragment) bit in the IP header when relaying the application data
/// onward to the peer and for determining the server capability in Allocate
/// requests. This attribute has no value part, and thus, the attribute length
/// field is 0.
pub struct DontFragment;

impl<'a> Attribute<'a> for DontFragment {
    type Error = StunError;
    type Item = ();

    const KIND: AttrKind = AttrKind::DontFragment;

    fn decode(_: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(())
    }
}
