use anyhow::Result;
use super::{Transaction, address, error};
use num_enum::TryFromPrimitive;
use std::cmp::{Eq, PartialEq};
use std::net::SocketAddr;
use bytes::BufMut;

/// message attribute type.
#[repr(u16)]
#[derive(Hash, Copy, Clone, Debug, TryFromPrimitive)]
pub enum Attribute {
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

impl Eq for Attribute {}
impl PartialEq for Attribute {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}

/// message attribute value.
#[derive(Clone, Debug)]
pub enum Value {
    UserName(String),
    Realm(String),
    Nonce(String),
    XorRelayedAddress(SocketAddr),
    XorMappedAddress(SocketAddr),
    MappedAddress(SocketAddr),
    ResponseOrigin(SocketAddr),
    Software(String),
    MessageIntegrity(String),
    ErrorCode(error::Error),
    Lifetime(u16)
}

impl Value {
    /// SocketAddr
    /// 添加填充位
    ///
    /// 协议规定需要填充0x00到头部.
    fn addr_handle(addr: SocketAddr, id: Transaction, xor: bool) -> Vec<u8> {
        let mut buffer = address::encoder(addr, id, xor);
        buffer.insert(0x00, 0);
        buffer
    }

    /// U16转Vec
    fn u16_as_vec(number: u16) -> Vec<u8> {
        let mut result = Vec::new();
        result.put_u16(number);
        result
    }

    /// 属性转缓冲区
    ///
    /// 将属性转换为缓冲区类型便于传输.
    #[rustfmt::skip]
    pub fn into_bytes(self, id: Transaction) -> Vec<u8> {
        match self {
            Self::UserName(username) => username.into_bytes(),
            Self::Realm(realm) => realm.into_bytes(),
            Self::Nonce(nonce) => nonce.into_bytes(),
            Self::XorRelayedAddress(addr) => Self::addr_handle(addr, id, true),
            Self::XorMappedAddress(addr) => Self::addr_handle(addr, id, true),
            Self::MappedAddress(addr) => Self::addr_handle(addr, id, false),
            Self::ResponseOrigin(addr) => Self::addr_handle(addr, id, false),
            Self::Software(value) => value.into_bytes(),
            Self::MessageIntegrity(value) => value.into_bytes(),
            Self::ErrorCode(value) => value.into_bytes(),
            Self::Lifetime(value) => Self::u16_as_vec(value),
        }
    }

    
}

impl Attribute {
    /// SocketAddr
    /// 删除填充位
    ///
    /// 移除头部的默认填充位.
    fn addr_handle(mut buffer: Vec<u8>, id: Transaction, xor: bool) -> Result<SocketAddr> {
        buffer.remove(0);
        Ok(address::decoder(buffer, id, xor)?)
    }

    /// 缓冲区转属性
    ///
    /// 将缓冲区转换为本地类型.
    #[rustfmt::skip]
    pub fn from(self, id: Transaction, value: Vec<u8>) -> Result<Value> {
        Ok(match self {
            Self::UserName => Value::UserName(String::from_utf8(value)?),
            Self::Realm => Value::Realm(String::from_utf8(value)?),
            Self::Nonce => Value::Nonce(String::from_utf8(value)?),
            Self::XorRelayedAddress => Value::XorRelayedAddress(Self::addr_handle(value, id, true)?),
            Self::XorMappedAddress => Value::XorMappedAddress(Self::addr_handle(value, id, true)?),
            Self::MappedAddress => Value::MappedAddress(Self::addr_handle(value, id, false)?),
            Self::ResponseOrigin => Value::ResponseOrigin(Self::addr_handle(value, id, false)?),
            Self::Software => Value::Software(String::from_utf8(value)?),
            Self::MessageIntegrity => Value::MessageIntegrity(String::from_utf8(value)?),
            Self::ErrorCode => Value::ErrorCode(error::Error::from(value)?),
            Self::Lifetime => Value::Lifetime(u16::from_be_bytes([ value[0], value[1] ])),
        })
    }
}
