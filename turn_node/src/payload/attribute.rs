use super::{util, Addr, Error};
use anyhow::Result;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

use bytes::{BufMut, BytesMut};

/// 属性类型
#[repr(u16)]
#[derive(TryFromPrimitive, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum AttrKind {
    UserName = 0x0006,
    Data = 0x0013,
    Realm = 0x0014,
    Nonce = 0x0015,
    XorPeerAddress = 0x0012,
    XorRelayedAddress = 0x0016,
    XorMappedAddress = 0x0020,
    MappedAddress = 0x0001,
    ResponseOrigin = 0x802B,
    Software = 0x8022,
    MessageIntegrity = 0x0008,
    ErrorAttrKind = 0x0009,
    Lifetime = 0x000D,
    ReqeestedTransport = 0x0019,
    Fingerprint = 0x8028,
    ChannelNumber = 0x000C,
}

/// 消息属性
#[derive(PartialEq, Eq, Debug)]
pub enum Property<'a> {
    Data(&'a [u8]),
    UserName(&'a str),
    Realm(&'a str),
    Nonce(&'a str),
    XorPeerAddress(Addr),
    XorRelayedAddress(Addr),
    XorMappedAddress(Addr),
    MappedAddress(Addr),
    ResponseOrigin(Addr),
    Software(&'a str),
    MessageIntegrity(&'a [u8]),
    ErrorCode(Error<'a>),
    Lifetime(u32),
    ReqeestedTransport,
    Fingerprint(u32),
    ChannelNumber(u16),
}

impl<'a> Property<'a> {
    /// 将属性序列化为缓冲区
    pub fn into_bytes(self, buf: &'a mut BytesMut, t: &[u8]) {
        match self {
            Self::UserName(u) => buf.put(u.as_bytes()),
            Self::Realm(r) => buf.put(r.as_bytes()),
            Self::Nonce(n) => buf.put(n.as_bytes()),
            Self::XorPeerAddress(addr) => addr.as_bytes(t, buf, true),
            Self::XorRelayedAddress(addr) => addr.as_bytes(t, buf, true),
            Self::XorMappedAddress(addr) => addr.as_bytes(t, buf, true),
            Self::MappedAddress(addr) => addr.as_bytes(t, buf, false),
            Self::ResponseOrigin(addr) => addr.as_bytes(t, buf, false),
            Self::Software(value) => buf.put(value.as_bytes()),
            Self::ErrorCode(value) => value.as_bytes(buf),
            Self::ReqeestedTransport => buf.put_u8(0x11),
            Self::Data(value) => buf.put(value),
            Self::ChannelNumber(v) => buf.put_u16(v),
            Self::Lifetime(v) => buf.put_u32(v),
            Self::MessageIntegrity(_) => (),
            Self::Fingerprint(_) => (),
        }
    }

    /// 根据属性获取属性类型
    pub fn attr(&self) -> AttrKind {
        match self {
            Self::UserName(_) => AttrKind::UserName,
            Self::Realm(_) => AttrKind::Realm,
            Self::Nonce(_) => AttrKind::Nonce,
            Self::XorPeerAddress(_) => AttrKind::XorPeerAddress,
            Self::XorMappedAddress(_) => AttrKind::XorMappedAddress,
            Self::XorRelayedAddress(_) => AttrKind::XorRelayedAddress,
            Self::MappedAddress(_) => AttrKind::MappedAddress,
            Self::ResponseOrigin(_) => AttrKind::ResponseOrigin,
            Self::Software(_) => AttrKind::Software,
            Self::MessageIntegrity(_) => AttrKind::MessageIntegrity,
            Self::ErrorCode(_) => AttrKind::ErrorAttrKind,
            Self::Lifetime(_) => AttrKind::Lifetime,
            Self::ReqeestedTransport => AttrKind::ReqeestedTransport,
            Self::Fingerprint(_) => AttrKind::Fingerprint,
            Self::ChannelNumber(_) => AttrKind::ChannelNumber,
            Self::Data(_) => AttrKind::Data,
        }
    }
}

impl AttrKind {
    /// 创建属性实例

    pub fn from<'a>(self, token: &[u8], v: &'a [u8]) -> Result<Property<'a>> {
        Ok(match self {
            Self::UserName => Property::UserName(Self::buf_as_str(v)?),
            Self::Realm => Property::Realm(Self::buf_as_str(v)?),
            Self::Nonce => Property::Nonce(Self::buf_as_str(v)?),
            Self::XorPeerAddress => Property::XorPeerAddress(Addr::try_from(v, token, true)?),
            Self::XorRelayedAddress => Property::XorRelayedAddress(Addr::try_from(v, token, true)?),
            Self::XorMappedAddress => Property::XorMappedAddress(Addr::try_from(v, token, true)?),
            Self::MappedAddress => Property::MappedAddress(Addr::try_from(v, token, false)?),
            Self::ResponseOrigin => Property::ResponseOrigin(Addr::try_from(v, token, false)?),
            Self::Fingerprint => Property::Fingerprint(util::as_u32(v)),
            Self::ChannelNumber => Property::ChannelNumber(util::as_u16(v)),
            Self::Software => Property::Software(Self::buf_as_str(v)?),
            Self::MessageIntegrity => Property::MessageIntegrity(v),
            Self::ErrorAttrKind => Property::ErrorCode(Error::try_from(v)?),
            Self::Lifetime => Property::Lifetime(util::as_u32(v)),
            Self::ReqeestedTransport => Property::ReqeestedTransport,
            Self::Data => Property::Data(v),
        })
    }

    fn buf_as_str(buffer: &[u8]) -> Result<&str> {
        Ok(std::str::from_utf8(buffer)?)
    }
}
