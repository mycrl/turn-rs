use bytes::{BufMut, BytesMut};
use std::convert::TryInto;
use std::sync::Arc;

use anyhow::{anyhow, Result};

use std::cmp::{Eq, PartialEq};

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// 协议类型
pub const FAMILY_IPV4: u8 = 0x01;
pub const FAMILY_IPV6: u8 = 0x02;

/// 协议地址
#[derive(Debug)]
pub struct Addr(pub Arc<SocketAddr>);
impl Addr {
    /// 将SocketAddr编码为缓冲区

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

    /// 将缓冲区解码为SocketAddr

    pub fn try_from(packet: &[u8], token: &[u8], is_xor: bool) -> Result<Self> {
        let port = u16::from_be_bytes([packet[2], packet[3]]);

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

/// 将缓冲区转为IpAddrV4
fn from_bytes_v4(packet: &[u8]) -> Result<IpAddr> {
    let buf: [u8; 4] = packet[4..8].try_into()?;
    Ok(IpAddr::V4(buf.into()))
}

/// 将缓冲区转为IpAddrV6
fn from_bytes_v6(packet: &[u8]) -> Result<IpAddr> {
    let buf: [u8; 16] = packet[4..20].try_into()?;
    Ok(IpAddr::V6(buf.into()))
}

/// XOR处理
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

fn xor(addr: &SocketAddr, token: &[u8]) -> SocketAddr {
    let port = addr.port() ^ (0x2112A442 >> 16) as u16;
    let ip_addr = match addr.ip() {
        IpAddr::V4(x) => xor_v4(x),
        IpAddr::V6(x) => xor_v6(x, token),
    };

    SocketAddr::new(ip_addr, port)
}

fn xor_v4(addr: Ipv4Addr) -> IpAddr {
    let mut octets = addr.octets();
    for (i, b) in octets.iter_mut().enumerate() {
        *b ^= (0x2112A442 >> (24 - i * 8)) as u8;
    }

    IpAddr::V4(From::from(octets))
}

fn xor_v6(addr: Ipv6Addr, token: &[u8]) -> IpAddr {
    let mut octets = addr.octets();
    for (i, b) in octets.iter_mut().enumerate().take(4) {
        *b ^= (0x2112A442 >> (24 - i * 8)) as u8;
    }

    for (i, b) in octets.iter_mut().enumerate().take(16).skip(4) {
        *b ^= token[i - 4];
    }

    IpAddr::V6(From::from(octets))
}
