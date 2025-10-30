pub mod address;
pub mod error;

use std::{fmt::Debug, net::SocketAddr};

use bytes::{Buf, BufMut};
use num_enum::TryFromPrimitive;

use super::{
    Error,
    attributes::{
        address::{IpFamily, XAddress},
        error::ErrorType,
    },
};

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
pub enum AttributeType {
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
    AccessToken = 0x001B,
    MessageIntegritySha256 = 0x001C,
    PasswordAlgorithm = 0x001D,
    UserHash = 0x001E,
    XorMappedAddress = 0x0020,
    ReservationToken = 0x0022,
    Priority = 0x0024,
    UseCandidate = 0x0025,
    Padding = 0x0026,
    ResponsePort = 0x0027,
    ConnectionId = 0x002A,
    AdditionalAddressFamily = 0x8000,
    AddressErrorCode = 0x8001,
    PasswordAlgorithms = 0x8002,
    AlternateDomain = 0x8003,
    Icmp = 0x8004,
    Software = 0x8022,
    AlternateServer = 0x8023,
    TransactionTransmitCounter = 0x8025,
    CacheTimeout = 0x8027,
    Fingerprint = 0x8028,
    IceControlled = 0x8029,
    IceControlling = 0x802A,
    ResponseOrigin = 0x802B,
    OtherAddress = 0x802C,
    EcnCheck = 0x802D,
    ThirdPartyAuathorization = 0x802E,
    MobilityTicket = 0x8030,
}

/// dyn stun/turn message attribute.
pub trait Attribute<'a> {
    type Error: Debug;

    /// current attribute inner type.
    type Item;

    /// current attribute type.
    const TYPE: AttributeType;

    /// write the current attribute to the bytesfer.
    #[allow(unused_variables)]
    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, transaction_id: &'a [u8]) {}

    /// convert bytesfer to current attribute.
    fn deserialize(bytes: &'a [u8], transaction_id: &'a [u8]) -> Result<Self::Item, Self::Error>;
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
#[derive(Debug, Clone, Copy)]
pub struct UserName;

impl<'a> Attribute<'a> for UserName {
    type Error = Error;
    type Item = &'a str;

    const TYPE: AttributeType = AttributeType::UserName;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put(value.as_bytes());
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(std::str::from_utf8(bytes)?)
    }
}

/// The USERHASH attribute is used as a replacement for the USERNAME attribute
/// when username anonymity is supported.
///
/// The value of USERHASH has a fixed length of 32 bytes.  The username MUST have
/// been processed using the OpaqueString profile [RFC8265], and the realm MUST
/// have been processed using the OpaqueString profile [RFC8265] before hashing.
///
/// The following is the operation that the client will perform to hash the username:
///
/// userhash = SHA-256(OpaqueString(username) ":" OpaqueString(realm))
#[derive(Debug, Clone, Copy)]
pub struct UserHash;

impl<'a> Attribute<'a> for UserHash {
    type Error = Error;
    type Item = &'a [u8];

    const TYPE: AttributeType = AttributeType::UserHash;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put(value);
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(bytes)
    }
}

/// The DATA attribute is present in all Send and Data indications.  The
/// value portion of this attribute is variable length and consists of
/// the application data (that is, the data that would immediately follow
/// the UDP header if the data was been sent directly between the client
/// and the peer).  If the length of this attribute is not a multiple of
/// 4, then padding must be added after this attribute.
#[derive(Debug, Clone, Copy)]
pub struct Data;

impl<'a> Attribute<'a> for Data {
    type Error = Error;
    type Item = &'a [u8];

    const TYPE: AttributeType = AttributeType::Data;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put(value);
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
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
#[derive(Debug, Clone, Copy)]
pub struct Realm;

impl<'a> Attribute<'a> for Realm {
    type Error = Error;
    type Item = &'a str;

    const TYPE: AttributeType = AttributeType::Realm;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put(value.as_bytes());
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
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
#[derive(Debug, Clone, Copy)]
pub struct Nonce;

impl<'a> Attribute<'a> for Nonce {
    type Error = Error;
    type Item = &'a str;

    const TYPE: AttributeType = AttributeType::Nonce;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put(value.as_bytes());
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
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
#[derive(Debug, Clone, Copy)]
pub struct Software;

impl<'a> Attribute<'a> for Software {
    type Error = Error;
    type Item = &'a str;

    const TYPE: AttributeType = AttributeType::Software;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put(value.as_bytes());
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
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
#[derive(Debug, Clone, Copy)]
pub struct MessageIntegrity;

impl<'a> Attribute<'a> for MessageIntegrity {
    type Error = Error;
    type Item = &'a [u8];

    const TYPE: AttributeType = AttributeType::MessageIntegrity;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put(value);
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(bytes)
    }
}

/// The MESSAGE-INTEGRITY-SHA256 attribute contains an HMAC-SHA256
/// [RFC2104] of the STUN message.  The MESSAGE-INTEGRITY-SHA256
/// attribute can be present in any STUN message type.  The MESSAGE-
/// INTEGRITY-SHA256 attribute contains an initial portion of the HMAC-
/// SHA-256 [RFC2104] of the STUN message.  The value will be at most 32
/// bytes, but it MUST be at least 16 bytes and MUST be a multiple of 4
/// bytes.  The value must be the full 32 bytes unless the STUN Usage
/// explicitly specifies that truncation is allowed.  STUN Usages may
/// specify a minimum length longer than 16 bytes.
///
/// The key for the HMAC depends on which credential mechanism is in use.
/// Section 9.1.1 defines the key for the short-term credential
/// mechanism, and Section 9.2.2 defines the key for the long-term
/// credential mechanism.  Other credential mechanism MUST define the key
/// that is used for the HMAC.
///
/// The text used as input to HMAC is the STUN message, up to and
/// including the attribute preceding the MESSAGE-INTEGRITY-SHA256
/// attribute.  The Length field of the STUN message header is adjusted
/// to point to the end of the MESSAGE-INTEGRITY-SHA256 attribute.  The
/// value of the MESSAGE-INTEGRITY-SHA256 attribute is set to a dummy
/// value.
///
/// Once the computation is performed, the value of the MESSAGE-
/// INTEGRITY-SHA256 attribute is filled in, and the value of the length
/// in the STUN header is set to its correct value -- the length of the
/// entire message.  Similarly, when validating the MESSAGE-INTEGRITY-
/// SHA256, the Length field in the STUN header must be adjusted to point
/// to the end of the MESSAGE-INTEGRITY-SHA256 attribute prior to
/// calculating the HMAC over the STUN message, up to and including the
/// attribute preceding the MESSAGE-INTEGRITY-SHA256 attribute.  Such
/// adjustment is necessary when attributes, such as FINGERPRINT, appear
/// after MESSAGE-INTEGRITY-SHA256.  See also Appendix B.1 for examples
/// of such calculations.
#[derive(Debug, Clone, Copy)]
pub struct MessageIntegritySha256;

impl<'a> Attribute<'a> for MessageIntegritySha256 {
    type Error = Error;
    type Item = &'a [u8];

    const TYPE: AttributeType = AttributeType::MessageIntegritySha256;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put(value);
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(bytes)
    }
}

/// The PASSWORD-ALGORITHM attribute is present only in requests.  It
/// contains the algorithm that the server must use to derive a key from
/// the long-term password.
///
/// The set of known algorithms is maintained by IANA.  The initial set
/// defined by this specification is found in Section 18.5.
///
/// The attribute contains an algorithm number and variable length
/// parameters.  The algorithm number is a 16-bit value as defined in
/// Section 18.5.  The parameters starts with the length (prior to
/// padding) of the parameters as a 16-bit value, followed by the
/// parameters that are specific to the algorithm.  The parameters are
/// padded to a 32-bit boundary, in the same manner as an attribute.
/// Similarly, the padding bits MUST be set to zero on sending and MUST
/// be ignored by the receiver.
///
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |          Algorithm           |  Algorithm Parameters Length   |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                    Algorithm Parameters (variable)
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PasswordAlgorithm {
    Md5 = 0x0001,
    Sha256 = 0x0002,
}

impl<'a> Attribute<'a> for PasswordAlgorithm {
    type Error = Error;
    type Item = Self;

    const TYPE: AttributeType = AttributeType::PasswordAlgorithm;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u16(value as u16);
        bytes.put_u16(0);
    }

    fn deserialize(mut bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        if bytes.len() < 4 {
            return Err(Error::InvalidInput);
        }

        let ty = match bytes.get_u16() {
            0x0001 => Self::Md5,
            0x0002 => Self::Sha256,
            _ => return Err(Error::InvalidInput),
        };

        // Ignore attribute value, as it does not exist currently
        let size = bytes.get_u16();
        bytes.advance(super::alignment_32(size as usize));

        Ok(ty)
    }
}

/// The PASSWORD-ALGORITHMS attribute may be present in requests and
/// responses.  It contains the list of algorithms that the server can
/// use to derive the long-term password.
///
/// The set of known algorithms is maintained by IANA.  The initial set
/// defined by this specification is found in Section 18.5.
///
/// The attribute contains a list of algorithm numbers and variable
/// length parameters.  The algorithm number is a 16-bit value as defined
/// in Section 18.5.  The parameters start with the length (prior to
/// padding) of the parameters as a 16-bit value, followed by the
/// parameters that are specific to each algorithm.  The parameters are
/// padded to a 32-bit boundary, in the same manner as an attribute.
pub struct PasswordAlgorithms;

impl<'a> Attribute<'a> for PasswordAlgorithms {
    type Error = Error;
    type Item = Vec<PasswordAlgorithm>;

    const TYPE: AttributeType = AttributeType::PasswordAlgorithms;

    fn serialize<B: BufMut>(algorithms: Self::Item, bytes: &mut B, transaction_id: &'a [u8]) {
        for algorithm in algorithms {
            PasswordAlgorithm::serialize(algorithm, bytes, transaction_id);
        }
    }

    fn deserialize(mut bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        let mut algorithms = Vec::new();

        loop {
            if bytes.len() < 4 {
                break;
            }

            let ty = match bytes.get_u16() {
                0x0001 => PasswordAlgorithm::Md5,
                0x0002 => PasswordAlgorithm::Sha256,
                _ => break,
            };

            // Ignore attribute value, as it does not exist currently
            let size = bytes.get_u16();
            bytes.advance(super::alignment_32(size as usize));

            algorithms.push(ty);
        }

        Ok(algorithms)
    }
}

/// [RFC5389]: https://datatracker.ietf.org/doc/html/rfc5389
///
/// The XOR-PEER-ADDRESS specifies the address and port of the peer as
/// seen from the TURN server.  (For example, the peer's server-reflexive
/// transport address if the peer is behind a NAT.)  It is encoded in the
/// same way as XOR-MAPPED-ADDRESS [RFC5389].
#[derive(Debug, Clone, Copy)]
pub struct XorPeerAddress;

impl<'a> Attribute<'a> for XorPeerAddress {
    type Error = Error;
    type Item = SocketAddr;

    const TYPE: AttributeType = AttributeType::XorPeerAddress;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, transaction_id: &'a [u8]) {
        XAddress::serialize(&value, transaction_id, bytes, true)
    }

    fn deserialize(bytes: &'a [u8], transaction_id: &'a [u8]) -> Result<Self::Item, Self::Error> {
        XAddress::deserialize(bytes, transaction_id, true)
    }
}

/// [RFC5389]: https://datatracker.ietf.org/doc/html/rfc5389
///
/// The XOR-RELAYED-ADDRESS is present in Allocate responses.  It
/// specifies the address and port that the server allocated to the
/// client.  It is encoded in the same way as XOR-MAPPED-ADDRESS
/// [RFC5389].
#[derive(Debug, Clone, Copy)]
pub struct XorRelayedAddress;

impl<'a> Attribute<'a> for XorRelayedAddress {
    type Error = Error;
    type Item = SocketAddr;

    const TYPE: AttributeType = AttributeType::XorRelayedAddress;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, transaction_id: &'a [u8]) {
        XAddress::serialize(&value, transaction_id, bytes, true)
    }

    fn deserialize(bytes: &'a [u8], transaction_id: &'a [u8]) -> Result<Self::Item, Self::Error> {
        XAddress::deserialize(bytes, transaction_id, true)
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
#[derive(Debug, Clone, Copy)]
pub struct XorMappedAddress;

impl<'a> Attribute<'a> for XorMappedAddress {
    type Error = Error;
    type Item = SocketAddr;

    const TYPE: AttributeType = AttributeType::XorMappedAddress;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, transaction_id: &'a [u8]) {
        XAddress::serialize(&value, transaction_id, bytes, true)
    }

    fn deserialize(bytes: &'a [u8], transaction_id: &'a [u8]) -> Result<Self::Item, Self::Error> {
        XAddress::deserialize(bytes, transaction_id, true)
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
#[derive(Debug, Clone, Copy)]
pub struct MappedAddress;

impl<'a> Attribute<'a> for MappedAddress {
    type Error = Error;
    type Item = SocketAddr;

    const TYPE: AttributeType = AttributeType::MappedAddress;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, transaction_id: &'a [u8]) {
        XAddress::serialize(&value, transaction_id, bytes, false)
    }

    fn deserialize(bytes: &'a [u8], transaction_id: &'a [u8]) -> Result<Self::Item, Self::Error> {
        XAddress::deserialize(bytes, transaction_id, false)
    }
}

/// The RESPONSE-ORIGIN attribute is inserted by the server and indicates
/// the source IP address and port the response was sent from.  It is
/// useful for detecting double NAT configurations.  It is only present
/// in Binding Responses.
#[derive(Debug, Clone, Copy)]
pub struct ResponseOrigin;

impl<'a> Attribute<'a> for ResponseOrigin {
    type Error = Error;
    type Item = SocketAddr;

    const TYPE: AttributeType = AttributeType::ResponseOrigin;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, transaction_id: &'a [u8]) {
        XAddress::serialize(&value, transaction_id, bytes, false)
    }

    fn deserialize(bytes: &'a [u8], transaction_id: &'a [u8]) -> Result<Self::Item, Self::Error> {
        XAddress::deserialize(bytes, transaction_id, false)
    }
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
#[derive(Debug, Clone, Copy)]
pub struct ErrorCode<'a> {
    pub code: u16,
    pub message: &'a str,
}

impl<'a> Attribute<'a> for ErrorCode<'a> {
    type Error = Error;
    type Item = Self;

    const TYPE: AttributeType = AttributeType::ErrorCode;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        value.serialize(bytes);
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Self::try_from(bytes)
    }
}

impl From<ErrorType> for ErrorCode<'_> {
    /// create error from error type.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turn_server::codec::message::attributes::error::ErrorType;
    ///
    /// // Error::from(ErrorType::TryAlternate);
    /// ```
    fn from(value: ErrorType) -> Self {
        Self {
            code: value as u16,
            message: value.into(),
        }
    }
}

impl ErrorCode<'_> {
    /// encode the error type as bytes.
    ///
    /// # Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use turn_server::codec::message::attributes::error::ErrorType;
    /// use turn_server::codec::Error;
    ///
    /// let buffer = [
    ///     0x00u8, 0x00, 0x03, 0x00, 0x54, 0x72, 0x79, 0x20, 0x41, 0x6c, 0x74,
    ///     0x65, 0x72, 0x6e, 0x61, 0x74, 0x65,
    /// ];
    ///
    /// let mut buf = BytesMut::with_capacity(1280);
    /// let error = turn_server::codec::message::attributes::ErrorCode::from(ErrorType::TryAlternate);
    ///
    /// error.serialize(&mut buf);
    /// assert_eq!(&buf[..], &buffer);
    /// ```
    pub fn serialize<B: BufMut>(self, bytes: &mut B) {
        bytes.put_u16(0x0000);
        bytes.put_u16(self.code);
        bytes.put(self.message.as_bytes());
    }
}

impl<'a> TryFrom<&'a [u8]> for ErrorCode<'a> {
    type Error = Error;

    /// # Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use turn_server::codec::message::attributes::error::ErrorType;
    /// use turn_server::codec::Error;
    ///
    /// let buffer = [
    ///     0x00u8, 0x00, 0x03, 0x00, 0x54, 0x72, 0x79, 0x20, 0x41, 0x6c, 0x74,
    ///     0x65, 0x72, 0x6e, 0x61, 0x74, 0x65,
    /// ];
    ///
    /// let error = turn_server::codec::message::attributes::ErrorCode::try_from(&buffer[..]).unwrap();
    /// assert_eq!(error.code, ErrorType::TryAlternate as u16);
    /// assert_eq!(error.message, "Try Alternate");
    /// ```
    fn try_from(packet: &'a [u8]) -> Result<Self, Self::Error> {
        if packet.len() < 4 {
            return Err(Error::InvalidInput);
        }

        if u16::from_be_bytes(packet[..2].try_into()?) != 0x0000 {
            return Err(Error::InvalidInput);
        }

        Ok(Self {
            code: u16::from_be_bytes(packet[2..4].try_into()?),
            message: std::str::from_utf8(&packet[4..])?,
        })
    }
}

impl Eq for ErrorCode<'_> {}
impl PartialEq for ErrorCode<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code
    }
}

/// The LIFETIME attribute represents the duration for which the server
/// will maintain an allocation in the absence of a refresh.  The value
/// portion of this attribute is 4-bytes long and consists of a 32-bit
/// unsigned integral value representing the number of seconds remaining
/// until expiration.
#[derive(Debug, Clone, Copy)]
pub struct Lifetime;

impl<'a> Attribute<'a> for Lifetime {
    type Error = Error;
    type Item = u32;

    const TYPE: AttributeType = AttributeType::Lifetime;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u32(value)
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
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
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
pub enum ReqeestedTransport {
    Tcp = 0x06000000,
    Udp = 0x11000000,
}

impl<'a> Attribute<'a> for ReqeestedTransport {
    type Error = Error;
    type Item = Self;

    const TYPE: AttributeType = AttributeType::ReqeestedTransport;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u32(value as u32)
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Self::try_from(u32::from_be_bytes(bytes.try_into()?)).map_err(|_| Error::InvalidInput)
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
#[derive(Debug, Clone, Copy)]
pub struct Fingerprint;

impl<'a> Attribute<'a> for Fingerprint {
    type Error = Error;
    type Item = u32;

    const TYPE: AttributeType = AttributeType::Fingerprint;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u32(value)
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u32::from_be_bytes(bytes.try_into()?))
    }
}

/// The CHANNEL-NUMBER attribute contains the number of the channel.  The
/// value portion of this attribute is 4 bytes long and consists of a
/// 16-bit unsigned integer followed by a two-octet RFFU (Reserved For
/// Future Use) field, which MUST be set to 0 on transmission and MUST be
/// ignored on reception.
#[derive(Debug, Clone, Copy)]
pub struct ChannelNumber;

impl<'a> Attribute<'a> for ChannelNumber {
    type Error = Error;
    type Item = u16;

    const TYPE: AttributeType = AttributeType::ChannelNumber;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u16(value)
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
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
#[derive(Debug, Clone, Copy)]
pub struct IceControlling;

impl<'a> Attribute<'a> for IceControlling {
    type Error = Error;
    type Item = u64;

    const TYPE: AttributeType = AttributeType::IceControlling;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u64(value)
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u64::from_be_bytes(bytes.try_into()?))
    }
}

/// The USE-CANDIDATE attribute indicates that the candidate pair
/// resulting from this check will be used for transmission of data.  The
/// attribute has no content (the Length field of the attribute is zero);
/// it serves as a flag.  It has an attribute value of 0x0025.
#[derive(Debug, Clone, Copy)]
pub struct UseCandidate;

impl<'a> Attribute<'a> for UseCandidate {
    type Error = Error;
    type Item = ();

    const TYPE: AttributeType = AttributeType::UseCandidate;

    fn serialize<B: BufMut>(_: Self::Item, _: &mut B, _: &'a [u8]) {}

    fn deserialize(_: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
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
#[derive(Debug, Clone, Copy)]
pub struct IceControlled;

impl<'a> Attribute<'a> for IceControlled {
    type Error = Error;
    type Item = u64;

    const TYPE: AttributeType = AttributeType::IceControlled;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u64(value)
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u64::from_be_bytes(bytes.try_into()?))
    }
}

/// The PRIORITY attribute indicates the priority that is to be
/// associated with a peer-reflexive candidate, if one will be discovered
/// by this check.  It is a 32-bit unsigned integer and has an attribute
/// value of 0x0024.
#[derive(Debug, Clone, Copy)]
pub struct Priority;

impl<'a> Attribute<'a> for Priority {
    type Error = Error;
    type Item = u32;

    const TYPE: AttributeType = AttributeType::Priority;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u32(value)
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u32::from_be_bytes(bytes.try_into()?))
    }
}

/// The RESERVATION-TOKEN attribute contains a transaction_id that uniquely identifies a
/// relayed transport address being held in reserve by the server. The server
/// includes this attribute in a success response to tell the client about the
/// transaction_id, and the client includes this attribute in a subsequent Allocate
/// request to request the server use that relayed transport address for the
/// allocation.
///
/// The attribute value is 8 bytes and contains the transaction_id value.
#[derive(Debug, Clone, Copy)]
pub struct ReservationToken;

impl<'a> Attribute<'a> for ReservationToken {
    type Error = Error;
    type Item = u64;

    const TYPE: AttributeType = AttributeType::ReservationToken;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u64(value)
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(u64::from_be_bytes(bytes.try_into()?))
    }
}

/// This attribute allows the client to request that the port in the relayed
/// transport address be even, and (optionally) that the server reserve the
/// next-higher port number.  The value portion of this attribute is 1 byte
/// long.
#[derive(Debug, Clone, Copy)]
pub struct EvenPort;

impl<'a> Attribute<'a> for EvenPort {
    type Error = Error;
    type Item = bool;

    const TYPE: AttributeType = AttributeType::EvenPort;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u8(if value { 0b10000000 } else { 0b00000000 })
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        if bytes.is_empty() {
            return Err(Error::InvalidInput);
        }

        Ok(bytes[0] == 0b10000000)
    }
}

/// The REQUESTED-ADDRESS-FAMILY attribute is used by clients to request the
/// allocation of a specific address type from a server.  The following is the
/// format of the REQUESTED-ADDRESS-FAMILY attribute. Note that TURN attributes
/// are TLV (Type-Length-Value) encoded, with a 16-bit type, a 16-bit length,
/// and a variable-length value.
#[derive(Debug, Clone, Copy)]
pub struct RequestedAddressFamily;

impl<'a> Attribute<'a> for RequestedAddressFamily {
    type Error = Error;
    type Item = IpFamily;

    const TYPE: AttributeType = AttributeType::RequestedAddressFamily;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u8(value as u8)
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        if bytes.is_empty() {
            return Err(Error::InvalidInput);
        }

        IpFamily::try_from(bytes[0]).map_err(|_| Error::InvalidInput)
    }
}

/// This attribute is used by clients to request the allocation of an IPv4 and
/// IPv6 address type from a server. It is encoded in the same way as the
/// REQUESTED-ADDRESS-FAMILY attribute; The ADDITIONAL-ADDRESS-FAMILY attribute
/// MAY be present in the Allocate request. The attribute value of 0x02 (IPv6
/// address) is the only valid value in Allocate request.
#[derive(Debug, Clone, Copy)]
pub struct AdditionalAddressFamily;

impl<'a> Attribute<'a> for AdditionalAddressFamily {
    type Error = Error;
    type Item = IpFamily;

    const TYPE: AttributeType = AttributeType::AdditionalAddressFamily;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put_u8(value as u8)
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        if bytes.is_empty() {
            return Err(Error::InvalidInput);
        }

        IpFamily::try_from(bytes[0]).map_err(|_| Error::InvalidInput)
    }
}

/// This attribute is used by the client to request that the server set the DF
/// (Don't Fragment) bit in the IP header when relaying the application data
/// onward to the peer and for determining the server capability in Allocate
/// requests. This attribute has no value part, and thus, the attribute length
/// field is 0.
#[derive(Debug, Clone, Copy)]
pub struct DontFragment;

impl<'a> Attribute<'a> for DontFragment {
    type Error = Error;
    type Item = ();

    const TYPE: AttributeType = AttributeType::DontFragment;

    fn deserialize(_: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(())
    }
}

/// This attribute is used by the STUN server to inform the client that
/// it supports third-party authorization.  This attribute value contains
/// the STUN server name.  The authorization server may have tie ups with
/// multiple STUN servers and vice versa, so the client MUST provide the
/// STUN server name to the authorization server so that it can select
/// the appropriate keying material to generate the self-contained transaction_id.
/// If the authorization server does not have tie up with the STUN
/// server, then it returns an error to the client.  If the client does
/// not support or is not capable of doing third-party authorization,
/// then it defaults to first-party authentication.  The
/// THIRD-PARTY-AUTHORIZATION attribute is a comprehension-optional
/// attribute (see Section 15 from [RFC5389]).  If the client is able to
/// comprehend THIRD-PARTY-AUTHORIZATION, it MUST ensure that third-party
/// authorization takes precedence over first-party authentication (as
/// explained in Section 10 of [RFC5389]).
#[derive(Debug, Clone, Copy)]
pub struct ThirdPartyAuathorization;

impl<'a> Attribute<'a> for ThirdPartyAuathorization {
    type Error = Error;
    type Item = &'a str;

    const TYPE: AttributeType = AttributeType::ThirdPartyAuathorization;

    fn serialize<B: BufMut>(value: Self::Item, bytes: &mut B, _: &'a [u8]) {
        bytes.put(value.as_bytes());
    }

    fn deserialize(bytes: &'a [u8], _: &'a [u8]) -> Result<Self::Item, Self::Error> {
        Ok(std::str::from_utf8(bytes)?)
    }
}
