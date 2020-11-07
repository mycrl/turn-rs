mod address;
mod attribute;
mod codec;
mod error;
mod util;

use num_enum::TryFromPrimitive;
use std::collections::HashMap;
use bytes::Bytes;
pub use attribute::{
    Attribute, 
    Code
};

use std::convert::{
    Into,
    TryFrom
};

/// Cookie
///
/// * 对于`RFC5389`为固定内容.
pub const MAGIC_COOKIE: u32 = 0x2112A442;

/// STUN消息.
#[derive(Debug)]
pub struct Message<'a> {
    pub flag: Flag,
    pub transaction: &'a [u8],
    reader: HashMap<Code, Attribute<'a>>,
    writer: Vec<(Code, Attribute<'a>)>,
}

/// 消息类型
#[repr(u16)]
#[derive(Copy, Clone, Debug, TryFromPrimitive)]
pub enum Flag {
    BindingReq = 0x0001,
    BindingRes = 0x0101,
    AllocateReq = 0x0003,
    AllocateRes = 0x0103,
    AllocateErrRes = 0x0113,
}

impl<'a> Message<'a> {
    /// 创建消息
    ///
    /// 指定消息类型和交易号创建空属性消息.
    pub fn new(
        flag: Flag, 
        transaction: &'a [u8]
    ) -> Self {
        Self {
            flag,
            transaction,
            reader: HashMap::new(),
            writer: Vec::new(),
        }
    }

    /// 添加属性
    ///
    /// 添加属性到消息中的属性列表.
    pub fn add_attr(&mut self, value: Attribute<'a>) {
        self.writer.push((value.into_code(), value))
    }

    /// 获取属性
    ///
    /// 从消息中的属性列表获取属性.
    pub fn get_attr(&self, key: Code) -> Option<&Attribute> {
        self.reader.get(&key)
    }
}

impl Into<Bytes> for Message<'_> {
    fn into(self) -> Bytes {
        codec::encoder(self)
    }
}

impl<'a> TryFrom<&'a [u8]> for Message<'a> {
    type Error = anyhow::Error;
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        codec::decoder(value)
    }
}