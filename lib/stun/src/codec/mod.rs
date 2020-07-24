mod decoder;
mod encoder;
mod net;

use anyhow::Result;
pub use decoder::decoder;
pub use encoder::encoder;
use num_enum::TryFromPrimitive;
use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;

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
    flag: Flag,
    transaction: Transaction,
    attributes: HashMap<Attributes, Attribute>,
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
}

impl Attribute {
    pub fn from(key: Attributes, value: Vec<u8>) -> Result<Self> {
        match key {
            Attributes::UserName => Ok(Attribute::UserName(String::from_utf8(value)?)),
            Attributes::Realm => Ok(Attribute::Realm(String::from_utf8(value)?)),
            Attributes::Nonce => Ok(Attribute::Nonce(String::from_utf8(value)?)),
        }
    }

    pub fn into(self) -> &'static [u8] {
        match self {
            Self::UserName(username) => username.as_bytes(),
            Self::Realm(realm) => realm.as_bytes(),
            Self::Nonce(nonce) => nonce.as_bytes(),
        }
    }
}
