use std::convert::TryInto;
use std::sync::Arc;
use bytes::{
    BufMut, 
    BytesMut
};

use anyhow::{
    anyhow, 
    ensure,
    Result
};

use std::cmp::{
    Eq, 
    PartialEq
};

use std::net::{
    IpAddr, 
    Ipv4Addr, 
    Ipv6Addr, 
    SocketAddr
};

/// ip family type.
pub const FAMILY_IPV4: u8 = 0x01;
pub const FAMILY_IPV6: u8 = 0x02;

#[derive(Debug)]
pub struct Addr(pub Arc<SocketAddr>);
impl Addr {
    /// encoder SocketAddr as Bytes.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use stun::address::*;
    /// use std::sync::Arc;
    /// use bytes::BytesMut;
    /// 
    /// let xor_addr_buf: [u8; 8] = [
    ///     0x00, 0x01, 0xfc, 0xbe,
    ///     0xe1, 0xba, 0xa4, 0x29
    /// ];
    /// 
    /// let addr_buf: [u8; 8] = [
    ///     0x00, 0x01, 0xdd, 0xac, 
    ///     0xc0, 0xa8, 0x00, 0x6b
    /// ];
    /// 
    /// let token: [u8; 12] = [
    ///     0x6c, 0x46, 0x62, 0x54,
    ///     0x75, 0x4b, 0x44, 0x51,
    ///      0x46, 0x48, 0x4c, 0x71
    /// ];
    /// 
    /// let source = "192.168.0.107:56748".parse().unwrap();
    /// let addr = Addr(Arc::new(source));
    /// 
    /// let mut buffer = BytesMut::with_capacity(1280);
    /// addr.as_bytes(&token, &mut buffer, true);
    /// assert_eq!(&xor_addr_buf, &buffer[..]);
    /// 
    /// let mut buffer = BytesMut::with_capacity(1280);
    /// addr.as_bytes(&token, &mut buffer, false);
    /// assert_eq!(&addr_buf, &buffer[..]);
    /// ```
    #[rustfmt::skip]
    pub fn as_bytes(&self, token: &[u8], buf: &mut BytesMut, is_xor: bool) {
        buf.put_u8(0);
        let xor_addr = if is_xor { 
            Arc::new(xor(self.0.as_ref(), token))
        } else { 
            self.0.clone()
        };

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
    /// use stun::address::*;
    /// use std::sync::Arc;
    /// 
    /// let xor_addr_buf: [u8; 8] = [
    ///     0x00, 0x01, 0xfc, 0xbe,
    ///     0xe1, 0xba, 0xa4, 0x29
    /// ];
    /// 
    /// let addr_buf: [u8; 8] = [
    ///     0x00, 0x01, 0xdd, 0xac, 
    ///     0xc0, 0xa8, 0x00, 0x6b
    /// ];
    /// 
    /// let token: [u8; 12] = [
    ///     0x6c, 0x46, 0x62, 0x54,
    ///     0x75, 0x4b, 0x44, 0x51,
    ///     0x46, 0x48, 0x4c, 0x71
    /// ];
    /// 
    /// let source = "192.168.0.107:56748"
    ///     .parse()
    ///     .unwrap();
    /// 
    /// let addr = Addr::try_from(&xor_addr_buf, &token, true).unwrap();
    /// assert_eq!(addr.addr(), Arc::new(source));
    /// 
    /// let addr = Addr::try_from(&addr_buf, &token, false).unwrap();
    /// assert_eq!(addr.addr(), Arc::new(source));
    /// ```
    #[rustfmt::skip]
    pub fn try_from(packet: &[u8], token: &[u8], is_xor: bool) -> Result<Self> {
        ensure!(packet.len() >= 4, "buf len < 4");
        let port = u16::from_be_bytes([
            packet[2], 
            packet[3]
        ]);

        let ip_addr = match packet[1] {
            FAMILY_IPV4 => from_bytes_v4(packet)?,
            FAMILY_IPV6 => from_bytes_v6(packet)?,
            _ => return Err(anyhow!("missing family")),
        };

        let dyn_addr = SocketAddr::new(ip_addr, port);
        let addr = Arc::new(if is_xor { 
            xor(&dyn_addr, token) 
        } else { 
            dyn_addr 
        });

        Ok(Self(addr))
    }

    /// get inner SocketAddr.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use stun::address::*;
    /// use std::sync::Arc;
    /// 
    /// let source = "192.168.0.107:56748".parse().unwrap();
    /// let addr = Addr(Arc::new(source));
    /// assert_eq!(addr.addr(), Arc::new(source));
    /// ```
    pub fn addr(&self) -> Arc<SocketAddr> {
        self.0.clone()
    }
}

impl Eq for Addr {}
impl PartialEq for Addr {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// Bytes as IpAddrV4.
fn from_bytes_v4(packet: &[u8]) -> Result<IpAddr> {
    ensure!(packet.len() >= 8, "buf len < 8");
    let buf: [u8; 4] = packet[4..8].try_into()?;
    Ok(IpAddr::V4(buf.into()))
}

/// Bytes as IpAddrV6.
fn from_bytes_v6(packet: &[u8]) -> Result<IpAddr> {
    ensure!(packet.len() >= 20, "buf len < 20");
    let buf: [u8; 16] = packet[4..20].try_into()?;
    Ok(IpAddr::V6(buf.into()))
}

/// XOR process
///
/// X-Port is computed by taking the mapped port in host byte order,
/// XOR'ing it with the most significant 16 bits of the magic cookie, and
/// then the converting the result to network byte order.  If the IP
/// address family is IPv4, X-Address is computed by taking the mapped IP
/// address in host byte order, XOR'ing it with the magic cookie, and
/// converting the result to network byte order.  If the IP address
/// family is IPv6, X-Address is computed by taking the mapped IP address
/// in host byte order, XOR'ing it with the concatenation of the magic
/// cookie and the 96-bit transaction ID, and converting the result to
/// network byte order.
#[rustfmt::skip]
fn xor(addr: &SocketAddr, token: &[u8]) -> SocketAddr {
    let port = addr.port() ^ (0x2112A442 >> 16) as u16;
    let ip_addr = match addr.ip() {
        IpAddr::V4(x) => xor_v4(x),
        IpAddr::V6(x) => xor_v6(x, token),
    };

    SocketAddr::new(
        ip_addr, 
        port
    )
}

#[rustfmt::skip]
fn xor_v4(addr: Ipv4Addr) -> IpAddr {
    let mut octets = addr.octets();
    for (i, b) in octets.iter_mut().enumerate() {
        *b ^= (0x2112A442 >> (24 - i * 8)) as u8;
    }

    IpAddr::V4(
        From::from(octets)
    )
}

#[rustfmt::skip]
fn xor_v6(addr: Ipv6Addr, token: &[u8]) -> IpAddr {
    let mut octets = addr.octets();
    for (i, b) in octets.iter_mut().enumerate().take(4) {
        *b ^= (0x2112A442 >> (24 - i * 8)) as u8;
    }

    for (i, b) in octets.iter_mut().enumerate().take(16).skip(4) {
        *b ^= token[i - 4];
    }

    IpAddr::V6(
        From::from(octets)
    )
}