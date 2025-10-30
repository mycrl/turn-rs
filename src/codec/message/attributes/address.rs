use std::net::{IpAddr, SocketAddr};

use bytes::{Buf, BufMut};
use num_enum::TryFromPrimitive;

use super::Error;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
pub enum IpFamily {
    V4 = 0x01,
    V6 = 0x02,
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
#[derive(Debug, Clone, Copy)]
pub struct XAddress;

impl XAddress {
    /// encoder SocketAddr as Bytes.
    ///
    /// # Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use turn_server::codec::message::attributes::address::XAddress;
    ///
    /// let xor_addr_bytes: [u8; 8] =
    ///     [0x00, 0x01, 0xfc, 0xbe, 0xe1, 0xba, 0xa4, 0x29];
    ///
    /// let addr_bytes: [u8; 8] = [0x00, 0x01, 0xdd, 0xac, 0xc0, 0xa8, 0x00, 0x6b];
    ///
    /// let transaction_id: [u8; 12] = [
    ///     0x6c, 0x46, 0x62, 0x54, 0x75, 0x4b, 0x44, 0x51, 0x46, 0x48, 0x4c, 0x71,
    /// ];
    ///
    /// let source = "192.168.0.107:56748".parse().unwrap();
    ///
    /// let mut buffer = BytesMut::with_capacity(1280);
    /// XAddress::serialize(&source, &transaction_id, &mut buffer, true);
    /// assert_eq!(&xor_addr_bytes, &buffer[..]);
    ///
    /// let mut buffer = BytesMut::with_capacity(1280);
    /// XAddress::serialize(&source, &transaction_id, &mut buffer, false);
    /// assert_eq!(&addr_bytes, &buffer[..]);
    /// ```
    pub fn serialize<B: BufMut>(
        addr: &SocketAddr,
        transaction_id: &[u8],
        bytes: &mut B,
        is_xor: bool,
    ) {
        bytes.put_u8(0);

        let xor_addr = if is_xor {
            xor(addr, transaction_id)
        } else {
            *addr
        };

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
    /// use turn_server::codec::message::attributes::address::XAddress;
    ///
    /// let xor_addr_bytes: [u8; 8] =
    ///     [0x00, 0x01, 0xfc, 0xbe, 0xe1, 0xba, 0xa4, 0x29];
    ///
    /// let addr_bytes: [u8; 8] = [0x00, 0x01, 0xdd, 0xac, 0xc0, 0xa8, 0x00, 0x6b];
    ///
    /// let transaction_id: [u8; 12] = [
    ///     0x6c, 0x46, 0x62, 0x54, 0x75, 0x4b, 0x44, 0x51, 0x46, 0x48, 0x4c, 0x71,
    /// ];
    ///
    /// let source = "192.168.0.107:56748".parse().unwrap();
    ///
    /// let addr = XAddress::deserialize(&xor_addr_bytes, &transaction_id, true).unwrap();
    /// assert_eq!(addr, source);
    ///
    /// let addr = XAddress::deserialize(&addr_bytes, &transaction_id, false).unwrap();
    /// assert_eq!(addr, source);
    /// ```
    pub fn deserialize(
        mut bytes: &[u8],
        transaction_id: &[u8],
        is_xor: bool,
    ) -> Result<SocketAddr, Error> {
        if bytes.len() < 4 {
            return Err(Error::InvalidInput);
        }

        // skip the first 8 bits
        bytes.advance(1);

        let family = IpFamily::try_from(bytes.get_u8()).map_err(|_| Error::InvalidInput)?;
        let port = bytes.get_u16();

        let addr = SocketAddr::new(
            match family {
                IpFamily::V4 => ipv4_from_bytes(bytes)?,
                IpFamily::V6 => ipv6_from_bytes(bytes)?,
            },
            port,
        );

        Ok(if is_xor {
            xor(&addr, transaction_id)
        } else {
            addr
        })
    }
}

/// # Test
///
/// ```
/// use std::net::IpAddr;
/// use turn_server::codec::message::attributes::address::ipv4_from_bytes;
///
/// let bytes: [u8; 4] = [0xc0, 0xa8, 0x00, 0x6b];
///
/// let source: IpAddr = "192.168.0.107".parse().unwrap();
///
/// let addr = ipv4_from_bytes(&bytes).unwrap();
/// assert_eq!(addr, source);
/// ```
pub fn ipv4_from_bytes(bytes: &[u8]) -> Result<IpAddr, Error> {
    if bytes.len() != 4 {
        return Err(Error::InvalidInput);
    }

    let bytes: [u8; 4] = bytes[..4].try_into()?;
    Ok(IpAddr::V4(bytes.into()))
}

/// # Test
///
/// ```
/// use std::net::IpAddr;
/// use turn_server::codec::message::attributes::address::ipv6_from_bytes;
///
/// let bytes: [u8; 16] = [
///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
///     0x00, 0x00, 0xFF, 0xFF, 0xC0, 0x0A, 0x2F, 0x0F,
/// ];
///
/// let source: IpAddr = "::ffff:192.10.47.15".parse().unwrap();
///
/// let addr = ipv6_from_bytes(&bytes).unwrap();
/// assert_eq!(addr, source);
/// ```
pub fn ipv6_from_bytes(bytes: &[u8]) -> Result<IpAddr, Error> {
    if bytes.len() != 16 {
        return Err(Error::InvalidInput);
    }

    let bytes: [u8; 16] = bytes[..16].try_into()?;
    Ok(IpAddr::V6(bytes.into()))
}

/// # Test
///
/// ```
/// use std::net::SocketAddr;
/// use turn_server::codec::message::attributes::address::xor;
///
/// let source: SocketAddr = "192.168.0.107:1".parse().unwrap();
///
/// let res: SocketAddr = "225.186.164.41:8467".parse().unwrap();
///
/// let transaction_id: [u8; 12] = [
///     0x6c, 0x46, 0x62, 0x54, 0x75, 0x4b, 0x44, 0x51, 0x46, 0x48, 0x4c, 0x71,
/// ];
///
/// let addr = xor(&source, &transaction_id);
/// assert_eq!(addr, res);
/// ```
pub fn xor(addr: &SocketAddr, transaction_id: &[u8]) -> SocketAddr {
    SocketAddr::new(
        match addr.ip() {
            IpAddr::V4(it) => {
                let mut octets = it.octets();
                for (i, b) in octets.iter_mut().enumerate() {
                    *b ^= (0x2112A442 >> (24 - i * 8)) as u8;
                }

                IpAddr::V4(From::from(octets))
            }
            IpAddr::V6(it) => {
                let mut octets = it.octets();
                for (i, b) in octets.iter_mut().enumerate().take(4) {
                    *b ^= (0x2112A442 >> (24 - i * 8)) as u8;
                }

                for (i, b) in octets.iter_mut().enumerate().take(16).skip(4) {
                    *b ^= transaction_id[i - 4];
                }

                IpAddr::V6(From::from(octets))
            }
        },
        addr.port() ^ (0x2112A442 >> 16) as u16,
    )
}
