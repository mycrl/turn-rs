mod decoder;
mod encoder;
mod net;

use anyhow::Result;
pub use decoder::decoder;
pub use encoder::encoder;
use num_enum::TryFromPrimitive;
use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::net::SocketAddr;

/// 交易ID
pub type Transaction = [u8; 12];

/// Cookie
pub const MAGIC_COOKIE: u32 = 0x2112A442;

/// 计算填充位
///
/// RFC5766规定属性内容是4的倍数，
/// 所以此处是为了计算出填充位的长度.
#[rustfmt::skip]
pub fn pad_size(size: usize) -> usize {
    let range = size % 4;
    if size == 0 || range == 0 { return 0; }
    4 - range
}

/// sutn message.
#[derive(Clone, Debug)]
pub struct Message {
    pub flag: Flag,
    pub transaction: Transaction,
    pub attributes: HashMap<Attributes, Attribute>,
}

/// message type.
#[repr(u16)]
#[derive(Copy, Clone, Debug, TryFromPrimitive)]
pub enum Flag {
    BindingRequest = 0x0001,
    BindingResponse = 0x0101,
    AllocateRequest = 0x0003,
    AllocateResponse = 0x0113,
}

/// message attributes.
#[repr(u16)]
#[derive(Hash, Copy, Clone, Debug, TryFromPrimitive)]
pub enum Attributes {
    UserName = 0x0006,
    Realm = 0x0014,
    Nonce = 0x0015,
    XorMappedAddress = 0x0020,
    MappedAddress = 0x0001,
    ResponseOrigin = 0x802B,
    Software = 0x8022,
}

impl Eq for Attributes {}
impl PartialEq for Attributes {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}

/// message attribute.
#[derive(Clone, Debug)]
pub enum Attribute {
    UserName(String),
    Realm(String),
    Nonce(String),
    XorMappedAddress(SocketAddr),
    MappedAddress(SocketAddr),
    ResponseOrigin(SocketAddr),
    Software(String),
}

impl Message {
    /// 创建消息
    ///
    /// 指定消息类型和交易号创建空属性类型.
    pub fn new(flag: Flag, transaction: Transaction) -> Self {
        Self {
            flag,
            transaction,
            attributes: HashMap::new(),
        }
    }

    /// 添加属性
    ///
    /// 添加属性到消息中的属性列表.
    pub fn add_attr(&mut self, key: Attributes, value: Attribute) -> bool {
        self.attributes.insert(key, value).is_some()
    }
}

impl Attribute {
    /// SocketAddr
    /// 添加填充位
    ///
    /// 协议规定需要填充0x00到头部.
    fn addr_handle(addr: &SocketAddr, id: Transaction) -> Vec<u8> {
        let mut buffer = net::encoder(&addr, id);
        buffer.insert(0x00, 0);
        buffer
    }

    /// 属性转缓冲区
    ///
    /// 将属性转换为缓冲器类型便于传输.
    #[rustfmt::skip]
    pub fn parse(self, id: Transaction) -> Vec<u8> {
        match self {
            Self::UserName(username) => username.into_bytes(),
            Self::Realm(realm) => realm.into_bytes(),
            Self::Nonce(nonce) => nonce.into_bytes(),
            Self::XorMappedAddress(addr) => Self::addr_handle(&addr, id),
            Self::MappedAddress(addr) => Self::addr_handle(&addr, id),
            Self::ResponseOrigin(addr) => Self::addr_handle(&addr, id),
            Self::Software(value) => value.into_bytes(),
        }
    }

    
}

impl Attributes {
    /// SocketAddr
    /// 删除填充位
    ///
    /// 移除头部的默认填充位.
    fn addr_handle(mut buffer: Vec<u8>, id: Transaction) -> Result<SocketAddr> {
        buffer.remove(0);
        Ok(net::decoder(buffer, id)?)
    }

    /// 缓冲区转属性
    ///
    /// 将缓冲区转换为本地类型.
    #[rustfmt::skip]
    pub fn from(self, id: Transaction, value: Vec<u8>) -> Result<Attribute> {
        Ok(match self {
            Self::UserName => Attribute::UserName(String::from_utf8(value)?),
            Self::Realm => Attribute::Realm(String::from_utf8(value)?),
            Self::Nonce => Attribute::Nonce(String::from_utf8(value)?),
            Self::XorMappedAddress => Attribute::XorMappedAddress(Self::addr_handle(value, id)?),
            Self::MappedAddress => Attribute::MappedAddress(Self::addr_handle(value, id)?),
            Self::ResponseOrigin => Attribute::ResponseOrigin(Self::addr_handle(value, id)?),
            Self::Software => Attribute::Software(String::from_utf8(value)?),
        })
    }
}
