use std::convert::TryInto;
use super::MAGIC_COOKIE;
use anyhow::{
    Result,
    anyhow
};

use bytes::{
    BufMut, 
    BytesMut,
    Bytes
};

use std::net::{
    IpAddr, 
    Ipv4Addr, 
    Ipv6Addr, 
    SocketAddr
};

/// 协议类型
pub const FAMILY_IPV4: u8 = 0x01;
pub const FAMILY_IPV6: u8 = 0x02;

/// 将SocketAddr编码为缓冲区  
///
/// * 通过指定是否进行XOR处理进行转换
///
/// # Examples
///
/// ```no_run
/// use super::decoder;
///
/// let source = "192.168.0.107:56748";
///
/// let xor_addr: [u8; 8] = [
///     0x00, 0x01, 0xfc, 0xbe, 
///     0xe1, 0xba, 0xa4, 0x29
/// ];
/// 
/// let id: [u8; 12] = [
///     0x6c, 0x46, 0x62, 0x54, 
///     0x75, 0x4b, 0x44, 0x51, 
///     0x46, 0x48, 0x4c, 0x71
/// ];
/// 
/// let buf = encoder(source.parse().unwrap(), &id, true);
/// assert_eq!(&xor_addr, &buf[..]);
/// ```
pub fn encoder(addr: SocketAddr, id: &[u8], is_xor: bool) -> Bytes {
    let mut packet = BytesMut::new();
    packet.put_u8(0);

    let xor_addr = if is_xor { 
        xor(&addr, id) 
    } else { 
        addr 
    };

    packet.put_u8(if xor_addr.is_ipv4() {
        FAMILY_IPV4
    } else {
        FAMILY_IPV6
    });
    
    packet.put_u16(xor_addr.port());
    if let IpAddr::V4(ip) = xor_addr.ip() {
        packet.put(&ip.octets()[..]);
    }
    
    if let IpAddr::V6(ip) = xor_addr.ip() {
        packet.put(&ip.octets()[..]);
    }
    
    packet.freeze()
}

/// 将缓冲区解码为SocketAddr
///
/// * 通过指定是否进行XOR处理进行转换
///
/// # Examples
///
/// ```no_run
/// use super::decoder;
/// 
/// let source = "192.168.0.107:56748";
///
/// let xor_addr: [u8; 8] = [
///     0x00, 0x01, 0xfc, 0xbe, 
///     0xe1, 0xba, 0xa4, 0x29
/// ];
/// 
/// let id: [u8; 12] = [
///     0x6c, 0x46, 0x62, 0x54, 
///     0x75, 0x4b, 0x44, 0x51, 
///     0x46, 0x48, 0x4c, 0x71
/// ];
/// 
/// let addr = decoder(&xor_addr, &id, true);
/// assert_eq!(addr.unwrap(), source.parse().unwrap());
/// ```
pub fn decoder(packet: &[u8], id: &[u8], is_xor: bool) -> Result<SocketAddr> {
    let port = u16::from_be_bytes([
        packet[2],
        packet[3]
    ]);

    let ip_addr = match packet[1] {
        FAMILY_IPV4 => from_bytes_v4(packet)?,
        FAMILY_IPV6 => from_bytes_v6(packet)?,
        _ => return Err(anyhow!("missing family"))
    };

    let addr = SocketAddr::new(ip_addr, port);
    Ok(if is_xor {
        xor(&addr, id)
    } else {
        addr
    })
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
///
fn xor(addr: &SocketAddr, id: &[u8]) -> SocketAddr {
    let port = addr.port() ^ (MAGIC_COOKIE >> 16) as u16;
    let ip_addr = match addr.ip() {
        IpAddr::V4(x) => xor_v4(x),
        IpAddr::V6(x) => xor_v6(x, id),
    };
    
    SocketAddr::new(
        ip_addr, 
        port
    )
}

/// 通过XOR函数处理IpAddrV4
fn xor_v4(addr: Ipv4Addr) -> IpAddr {
    let mut octets = addr.octets();
    for (i, b) in octets.iter_mut().enumerate() {
        *b ^= (MAGIC_COOKIE >> (24 - i * 8)) as u8;
    }
        
    IpAddr::V4(
        From::from(octets)
    )
}

/// 通过XOR函数处理IpAddrV6
fn xor_v6(addr: Ipv6Addr, id: &[u8]) -> IpAddr {
    let mut octets = addr.octets();
    for (i, b) in octets.iter_mut().enumerate().take(4) {
        *b ^= (MAGIC_COOKIE >> (24 - i * 8)) as u8;
    }

    for (i, b) in octets.iter_mut().enumerate().take(16).skip(4) {
        *b ^= id[i - 4];
    }
    
    IpAddr::V6(
        From::from(octets)
    )
}