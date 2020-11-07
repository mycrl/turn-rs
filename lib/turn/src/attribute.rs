use num_enum::TryFromPrimitive;
use anyhow::Result;
use bytes::Bytes;
use super::{
    error,
    address
};

use std::cmp::{ 
    PartialEq,
    Eq
};

use std::{
    net::SocketAddr,
    convert::TryFrom
};

/// 属性类型
#[repr(u16)]
#[derive(Copy, Clone, Hash, Debug, TryFromPrimitive)]
pub enum Code {
    UserName = 0x0006,
    Realm = 0x0014,
    Nonce = 0x0015,
    XorRelayedAddress = 0x0016,
    XorMappedAddress = 0x0020,
    MappedAddress = 0x0001,
    ResponseOrigin = 0x802B,
    Software = 0x8022,
    MessageIntegrity = 0x0008,
    ErrorCode = 0x0009,
    Lifetime = 0x000D,
}

/// 消息属性
#[derive(Clone, Debug)]
pub enum Attribute<'a> {
    UserName(&'a str),
    Realm(&'a str),
    Nonce(&'a str),
    XorRelayedAddress(SocketAddr),
    XorMappedAddress(SocketAddr),
    MappedAddress(SocketAddr),
    ResponseOrigin(SocketAddr),
    Software(&'a str),
    MessageIntegrity(&'a str),
    ErrorCode(error::Error<'a>),
    Lifetime(u16),
}

impl<'a> Attribute<'a> {
    pub fn into_bytes(self, id: &[u8]) -> Bytes {
        match self {
            Self::UserName(username) => Bytes::copy_from_slice(username.as_bytes()),
            Self::Realm(realm) => Bytes::copy_from_slice(realm.as_bytes()),
            Self::Nonce(nonce) => Bytes::copy_from_slice(nonce.as_bytes()),
            Self::XorRelayedAddress(addr) => address::encoder(addr, id, true),
            Self::XorMappedAddress(addr) => address::encoder(addr, id, true),
            Self::MappedAddress(addr) => address::encoder(addr, id, false),
            Self::ResponseOrigin(addr) => address::encoder(addr, id, false),
            Self::Software(value) => Bytes::copy_from_slice(value.as_bytes()),
            Self::MessageIntegrity(value) => Bytes::copy_from_slice(value.as_bytes()),
            Self::Lifetime(value) => Bytes::copy_from_slice(&value.to_be_bytes()),
            Self::ErrorCode(value) => value.as_bytes(),
        }
    }

    pub fn into_code(&self) -> Code {
        match self {
            Self::UserName(_) => Code::UserName,
            Self::Realm(_) => Code::Realm,
            Self::Nonce(_) => Code::Nonce,
            Self::XorMappedAddress(_) => Code::XorMappedAddress,
            Self::XorRelayedAddress(_) => Code::XorRelayedAddress,
            Self::MappedAddress(_) => Code::MappedAddress,
            Self::ResponseOrigin(_) => Code::ResponseOrigin,
            Self::Software(_) => Code::Software,
            Self::MessageIntegrity(_) => Code::MessageIntegrity,
            Self::ErrorCode(_) => Code::ErrorCode,
            Self::Lifetime(_) => Code::Lifetime,
        }
    }
}

impl Code {
    #[rustfmt::skip]
    pub fn from<'a>(self, id: &'a [u8], value: &'a [u8]) -> Result<Attribute<'a>> {
        Ok(match self {
            Self::UserName => Attribute::UserName(Self::into(value)),
            Self::Realm => Attribute::Realm(Self::into(value)),
            Self::Nonce => Attribute::Nonce(Self::into(value)),
            Self::XorRelayedAddress => Attribute::XorRelayedAddress(address::decoder(value, id, true)?),
            Self::XorMappedAddress => Attribute::XorMappedAddress(address::decoder(value, id, true)?),
            Self::MappedAddress => Attribute::MappedAddress(address::decoder(value, id, false)?),
            Self::ResponseOrigin => Attribute::ResponseOrigin(address::decoder(value, id, false)?),
            Self::Software => Attribute::Software(Self::into(value)),
            Self::MessageIntegrity => Attribute::MessageIntegrity(Self::into(value)),
            Self::ErrorCode => Attribute::ErrorCode(error::Error::try_from(value)?),
            Self::Lifetime => Attribute::Lifetime(u16::from_be_bytes([value[0], value[1]])),
        })
    }

    fn into<'a>(buffer: &'a [u8]) -> &'a str {
        unsafe {
            std::mem::transmute::<&[u8], &str>(buffer)
        }
    }
}

impl Eq for Code {}
impl PartialEq for Code {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}


