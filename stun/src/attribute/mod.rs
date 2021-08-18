mod address;
mod error;

use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use std::net::SocketAddr;
pub use address::Addr;
use crate::util;
use bytes::{
    BytesMut,
    BufMut
};

pub use error::{
    Kind as ErrKind,
    Error
};

/// attribute type.
#[repr(u16)]
#[derive(TryFromPrimitive)]
#[derive(PartialEq, Eq, Hash)]
pub enum AttrKind {
    UserName            = 0x0006,
    Data                = 0x0013,
    Realm               = 0x0014,
    Nonce               = 0x0015,
    XorPeerAddress      = 0x0012,
    XorRelayedAddress   = 0x0016,
    XorMappedAddress    = 0x0020,
    MappedAddress       = 0x0001,
    ResponseOrigin      = 0x802B,
    Software            = 0x8022,
    MessageIntegrity    = 0x0008,
    ErrorCode           = 0x0009,
    Lifetime            = 0x000D,
    ReqeestedTransport  = 0x0019,
    Fingerprint         = 0x8028,
    ChannelNumber       = 0x000C,
}

/// dyn stun/turn message attribute.
pub trait Property<'a> {
    type Error;
    // current attribute inner type.
    type Inner;
    /// get current attribute type.
    fn kind() -> AttrKind;
    /// write the current attribute to the buffer.
    fn into(value: Self::Inner, buf: &mut BytesMut, t: &'a [u8]);
    /// convert buffer to current attribute.
    fn try_from(buf: &'a [u8], t: &'a [u8]) -> Result<Self::Inner, Self::Error>;
}

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
impl<'a> Property<'a> for UserName {
    type Inner = &'a str;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::UserName
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, _: &[u8]) {
        buf.put(value.as_bytes());
    }

    fn try_from(buf: &'a [u8], _: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Ok(std::str::from_utf8(buf)?)
    }
}

/// The DATA attribute is present in all Send and Data indications.  The
/// value portion of this attribute is variable length and consists of
/// the application data (that is, the data that would immediately follow
/// the UDP header if the data was been sent directly between the client
/// and the peer).  If the length of this attribute is not a multiple of
/// 4, then padding must be added after this attribute.
pub struct Data;
impl<'a> Property<'a> for Data {
    type Inner = &'a [u8];
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::Data
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, _: &[u8]) {
        buf.put(value);
    }

    fn try_from(buf: &'a [u8], _: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Ok(buf)
    }
}

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
impl<'a> Property<'a> for Realm {
    type Inner = &'a str;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::Realm
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, _: &[u8]) {
        buf.put(value.as_bytes());
    }

    fn try_from(buf: &'a [u8], _: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Ok(std::str::from_utf8(buf)?)
    }
}

/// The NONCE attribute may be present in requests and responses.  It
/// contains a sequence of qdtext or quoted-pair, which are defined in
/// [RFC3261].  Note that this means that the NONCE attribute will not
/// contain the actual surrounding quote characters.  The NONCE attribute
/// MUST be fewer than 128 characters (which can be as long as 509 bytes
/// when encoding them and a long as 763 bytes when decoding them).  See
/// Section 5.4 of [RFC7616] for guidance on selection of nonce values in
/// a server.
pub struct Nonce;
impl<'a> Property<'a> for Nonce {
    type Inner = &'a str;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::Nonce
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, _: &[u8]) {
        buf.put(value.as_bytes());
    }

    fn try_from(buf: &'a [u8], _: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Ok(std::str::from_utf8(buf)?)
    }
}

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
impl<'a> Property<'a> for Software {
    type Inner = &'a str;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::Software
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, _: &[u8]) {
        buf.put(value.as_bytes());
    }

    fn try_from(buf: &'a [u8], _: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Ok(std::str::from_utf8(buf)?)
    }
}

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
impl<'a> Property<'a> for MessageIntegrity {
    type Inner = &'a [u8];
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::MessageIntegrity
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, _: &[u8]) {
        buf.put(value);
    }

    fn try_from(buf: &'a [u8], _: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Ok(buf)
    }
}

/// The XOR-PEER-ADDRESS specifies the address and port of the peer as
/// seen from the TURN server.  (For example, the peer's server-reflexive
/// transport address if the peer is behind a NAT.)  It is encoded in the
/// same way as XOR-MAPPED-ADDRESS [RFC5389].
pub struct XorPeerAddress;
impl<'a> Property<'a> for XorPeerAddress {
    type Inner = SocketAddr;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::XorPeerAddress
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, token: &[u8]) {
        Addr::into(&value, token, buf, true)
    }

    fn try_from(buf: &'a [u8], token: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Addr::try_from(buf, token, true)
    }
}

/// The XOR-RELAYED-ADDRESS is present in Allocate responses.  It
/// specifies the address and port that the server allocated to the
/// client.  It is encoded in the same way as XOR-MAPPED-ADDRESS
/// [RFC5389].
pub struct XorRelayedAddress;
impl<'a> Property<'a> for XorRelayedAddress {
    type Inner = SocketAddr;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::XorRelayedAddress
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, token: &[u8]) {
        Addr::into(&value, token, buf, true)
    }

    fn try_from(buf: &'a [u8], token: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Addr::try_from(buf, token, true)
    }
}

/// The XOR-MAPPED-ADDRESS attribute is identical to the MAPPED-ADDRESS
/// attribute, except that the reflexive transport address is obfuscated
/// through the XOR function.
/// 
/// The format of the XOR-MAPPED-ADDRESS is:
/// 
/// ```bash
///   0                   1                   2                   3
///   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |0 0 0 0 0 0 0 0|    Family     |         X-Port                |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |                X-Address (Variable)
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
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
/// directly in binary.  RFC 3489 originally specified only MAPPED-
/// ADDRESS.  However, deployment experience found that some NATs rewrite
/// the 32-bit binary payloads containing the NAT's public IP address,
/// such as STUN's MAPPED-ADDRESS attribute, in the well-meaning but
/// misguided attempt to provide a generic Application Layer Gateway
/// (ALG) function.  Such behavior interferes with the operation of STUN
/// and also causes failure of STUN's message-integrity checking.
pub struct XorMappedAddress;
impl<'a> Property<'a> for XorMappedAddress {
    type Inner = SocketAddr;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::XorMappedAddress
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, token: &[u8]) {
        Addr::into(&value, token, buf, true)
    }

    fn try_from(buf: &'a [u8], token: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Addr::try_from(buf, token, true)
    }
}

/// The MAPPED-ADDRESS attribute indicates a reflexive transport address
/// of the client.  It consists of an 8-bit address family and a 16-bit
/// port, followed by a fixed-length value representing the IP address.
/// If the address family is IPv4, the address MUST be 32 bits.  If the
/// address family is IPv6, the address MUST be 128 bits.  All fields
/// must be in network byte order.
/// 
/// The format of the MAPPED-ADDRESS attribute is:
/// 
/// ```bash
///   0                   1                   2                   3
///   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |0 0 0 0 0 0 0 0|    Family     |           Port                |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |                                                               |
///  |                 Address (32 bits or 128 bits)                 |
///  |                                                               |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
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
impl<'a> Property<'a> for MappedAddress {
    type Inner = SocketAddr;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::MappedAddress
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, token: &[u8]) {
        Addr::into(&value, token, buf, false)
    }

    fn try_from(buf: &'a [u8], token: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Addr::try_from(buf, token, false)
    }
}

/// The RESPONSE-ORIGIN attribute is inserted by the server and indicates
/// the source IP address and port the response was sent from.  It is
/// useful for detecting double NAT configurations.  It is only present
/// in Binding Responses.
pub struct ResponseOrigin;
impl<'a> Property<'a> for ResponseOrigin {
    type Inner = SocketAddr;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::ResponseOrigin
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, token: &[u8]) {
        Addr::into(&value, token, buf, false)
    }

    fn try_from(buf: &'a [u8], token: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Addr::try_from(buf, token, false)
    }
}

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
/// ```bash
///   0                   1                   2                   3
///   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |           Reserved, should be 0         |Class|     Number    |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |      Reason Phrase (variable)                                ..
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
pub struct ErrorCode;
impl<'a> Property<'a> for ErrorCode {
    type Inner = Error<'a>;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::ErrorCode
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, _: &[u8]) {
        value.into(buf)
    }

    fn try_from(buf: &'a [u8], _: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Error::try_from(buf)
    }
}

/// The LIFETIME attribute represents the duration for which the server
/// will maintain an allocation in the absence of a refresh.  The value
/// portion of this attribute is 4-bytes long and consists of a 32-bit
/// unsigned integral value representing the number of seconds remaining
/// until expiration.
pub struct Lifetime;
impl<'a> Property<'a> for Lifetime {
    type Inner = u32;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::Lifetime
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, _: &[u8]) {
        buf.put_u32(value)
    }

    fn try_from(buf: &'a [u8], _: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Ok(util::as_u32(buf))
    }
}

/// This attribute is used by the client to request a specific transport
/// protocol for the allocated transport address.  The value of this
/// attribute is 4 bytes with the following format:
///
/// ```bash
///   0                   1                   2                   3
///   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |    Protocol   |                    RFFU                       |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
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
impl<'a> Property<'a> for ReqeestedTransport {
    type Inner = u8;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::ReqeestedTransport
    }

    fn into(_: Self::Inner, buf: &mut BytesMut, _: &[u8]) {
        buf.put_u8(0x11)
    }

    fn try_from(buf: &'a [u8], _: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Ok(buf[0])
    }
}

/// The FINGERPRINT attribute MAY be present in all STUN messages.
/// 
/// The value of the attribute is computed as the CRC-32 of the STUN
/// message up to (but excluding) the FINGERPRINT attribute itself,
/// XOR'ed with the 32-bit value 0x5354554e.  (The XOR operation ensures
/// that the FINGERPRINT test will not report a false positive on a
/// packet containing a CRC-32 generated by an application protocol.)
/// The 32-bit CRC is the one defined in ITU V.42 [ITU.V42.2002], which
/// has a generator polynomial of x^32 + x^26 + x^23 + x^22 + x^16 + x^12
/// + x^11 + x^10 + x^8 + x^7 + x^5 + x^4 + x^2 + x + 1.  See the sample
/// code for the CRC-32 in Section 8 of [RFC1952].
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
impl<'a> Property<'a> for Fingerprint {
    type Inner = u32;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::Fingerprint
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, _: &[u8]) {
        buf.put_u32(value)
    }

    fn try_from(buf: &'a [u8], _: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Ok(util::as_u32(buf))
    }
}

/// The CHANNEL-NUMBER attribute contains the number of the channel.  The
/// value portion of this attribute is 4 bytes long and consists of a
/// 16-bit unsigned integer followed by a two-octet RFFU (Reserved For
/// Future Use) field, which MUST be set to 0 on transmission and MUST be
/// ignored on reception.
/// 
/// ```bash
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |        Channel Number         |         RFFU = 0              |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
pub struct ChannelNumber;
impl<'a> Property<'a> for ChannelNumber {
    type Inner = u16;
    type Error = anyhow::Error;
    fn kind() -> AttrKind {
        AttrKind::ChannelNumber
    }

    fn into(value: Self::Inner, buf: &mut BytesMut, _: &[u8]) {
        buf.put_u16(value)
    }

    fn try_from(buf: &'a [u8], _: &'a [u8]) -> Result<Self::Inner, Self::Error> {
        Ok(util::as_u16(buf))
    }
}
