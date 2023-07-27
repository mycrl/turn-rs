use anyhow::{anyhow, ensure, Result};
use bytes::{BufMut, BytesMut};

use std::convert::TryInto;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub const FAMILY_IPV4: u8 = 0x01;
pub const FAMILY_IPV6: u8 = 0x02;

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
    /// # Unit Test
    ///
    /// ```
    /// use faster_stun::attribute::*;
    /// use bytes::BytesMut;
    ///
    /// let xor_addr_buf: [u8; 8] = [0x00, 0x01, 0xfc, 0xbe, 0xe1, 0xba, 0xa4, 0x29];
    ///
    /// let addr_buf: [u8; 8] = [0x00, 0x01, 0xdd, 0xac, 0xc0, 0xa8, 0x00, 0x6b];
    ///
    /// let token: [u8; 12] = [
    ///     0x6c, 0x46, 0x62, 0x54, 0x75, 0x4b, 0x44, 0x51, 0x46, 0x48, 0x4c, 0x71,
    /// ];
    ///
    /// let source = "192.168.0.107:56748".parse().unwrap();
    ///
    /// let mut buffer = BytesMut::with_capacity(1280);
    /// Addr::into(&source, &token, &mut buffer, true);
    /// assert_eq!(&xor_addr_buf, &buffer[..]);
    ///
    /// let mut buffer = BytesMut::with_capacity(1280);
    /// Addr::into(&source, &token, &mut buffer, false);
    /// assert_eq!(&addr_buf, &buffer[..]);
    /// ```
    pub fn into(a: &SocketAddr, token: &[u8], buf: &mut BytesMut, is_xor: bool) {
        buf.put_u8(0);
        let xor_addr = if is_xor { xor(a, token) } else { *a };

        buf.put_u8(if xor_addr.is_ipv4() {
            FAMILY_IPV4
        } else {
            FAMILY_IPV6
        });

        buf.put_u16(xor_addr.port());
        if let IpAddr::V4(ip) = xor_addr.ip() {
            buf.put(&ip.octets()[..]);
        }

        if let IpAddr::V6(ip) = xor_addr.ip() {
            buf.put(&ip.octets()[..]);
        }
    }

    /// decoder Bytes as SocketAddr.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use faster_stun::attribute::*;
    ///
    /// let xor_addr_buf: [u8; 8] = [0x00, 0x01, 0xfc, 0xbe, 0xe1, 0xba, 0xa4, 0x29];
    ///
    /// let addr_buf: [u8; 8] = [0x00, 0x01, 0xdd, 0xac, 0xc0, 0xa8, 0x00, 0x6b];
    ///
    /// let token: [u8; 12] = [
    ///     0x6c, 0x46, 0x62, 0x54, 0x75, 0x4b, 0x44, 0x51, 0x46, 0x48, 0x4c, 0x71,
    /// ];
    ///
    /// let source = "192.168.0.107:56748".parse().unwrap();
    ///
    /// let addr = Addr::try_from(&xor_addr_buf, &token, true).unwrap();
    /// assert_eq!(addr, source);
    ///
    /// let addr = Addr::try_from(&addr_buf, &token, false).unwrap();
    /// assert_eq!(addr, source);
    /// ```
    pub fn try_from(packet: &[u8], token: &[u8], is_xor: bool) -> Result<SocketAddr> {
        ensure!(packet.len() >= 4, "buf len < 4");
        let port = u16::from_be_bytes([packet[2], packet[3]]);

        let ip_addr = match packet[1] {
            FAMILY_IPV4 => from_bytes_v4(packet)?,
            FAMILY_IPV6 => from_bytes_v6(packet)?,
            _ => return Err(anyhow!("missing family!")),
        };

        let dyn_addr = SocketAddr::new(ip_addr, port);
        Ok(if is_xor {
            xor(&dyn_addr, token)
        } else {
            dyn_addr
        })
    }
}

/// # Unit Test
///
/// ```
/// use faster_stun::attribute::address::*;
/// use std::net::IpAddr;
///
/// let buf: [u8; 8] = [0x00, 0x01, 0xdd, 0xac, 0xc0, 0xa8, 0x00, 0x6b];
///
/// let source: IpAddr = "192.168.0.107".parse().unwrap();
///
/// let addr = from_bytes_v4(&buf).unwrap();
/// assert_eq!(addr, source);
/// ```
pub fn from_bytes_v4(packet: &[u8]) -> Result<IpAddr> {
    ensure!(packet.len() == 8, "invalid ip addr!");
    let buf: [u8; 4] = packet[4..8].try_into()?;
    Ok(IpAddr::V4(buf.into()))
}

/// # Unit Test
///
/// ```
/// use faster_stun::attribute::address::*;
/// use std::net::IpAddr;
///
/// let buf: [u8; 20] = [
///     0x00, 0x01, 0xdd, 0xac, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
///     0xFF, 0xC0, 0x0A, 0x2F, 0x0F,
/// ];
///
/// let source: IpAddr = "::ffff:192.10.47.15".parse().unwrap();
///
/// let addr = from_bytes_v6(&buf).unwrap();
/// assert_eq!(addr, source);
/// ```
pub fn from_bytes_v6(packet: &[u8]) -> Result<IpAddr> {
    ensure!(packet.len() == 20, "invalid ip addr!");
    let buf: [u8; 16] = packet[4..20].try_into()?;
    Ok(IpAddr::V6(buf.into()))
}

/// # Unit Test
///
/// ```
/// use faster_stun::attribute::address::*;
/// use std::net::SocketAddr;
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

/// # Unit Test
///
/// ```
/// use faster_stun::attribute::address::*;
/// use std::net::{
///     Ipv4Addr,
///     IpAddr,
/// };
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

/// # Unit Test
///
/// ```
/// use faster_stun::attribute::address::*;
/// use std::net::{
///     Ipv6Addr,
///     IpAddr,
/// };
///
/// let source: Ipv6Addr = "::ffff:192.10.47.15".parse().unwrap();
///
/// let xor: IpAddr = "2112:a442:6c46:6254:754b:bbae:8642:637e".parse().unwrap();
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
