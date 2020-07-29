mod decoder;
mod encoder;
mod address;
pub mod attribute;
pub mod error;
pub mod util;

pub use decoder::decoder;
pub use encoder::encoder;
use num_enum::TryFromPrimitive;
use std::collections::HashMap;
use attribute::{Attribute, Value};

/// 交易ID
pub type Transaction = [u8; 12];

/// Cookie
pub const MAGIC_COOKIE: u32 = 0x2112A442;

/// sutn message.
#[derive(Clone, Debug)]
pub struct Message {
    pub flag: Flag,
    pub transaction: Transaction,
    pub attributes: HashMap<Attribute, Value>,
}

/// message type.
#[repr(u16)]
#[derive(Copy, Clone, Debug, TryFromPrimitive)]
pub enum Flag {
    BindingReq = 0x0001,
    BindingRes = 0x0101,
    AllocateReq = 0x0003,
    AllocateRes = 0x0103,
    AllocateErrRes = 0x0113,
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
    pub fn add_attr(&mut self, value: Value) -> bool {
        self.attributes.insert(match &value {
            Value::UserName(_) => Attribute::UserName,
            Value::Realm(_) => Attribute::Realm,
            Value::Nonce(_) => Attribute::Nonce,
            Value::XorRelayedAddress(_) => Attribute::XorRelayedAddress,
            Value::XorMappedAddress(_) => Attribute::XorMappedAddress,
            Value::MappedAddress(_) => Attribute::MappedAddress,
            Value::ResponseOrigin(_) => Attribute::ResponseOrigin,
            Value::Software(_) => Attribute::Software,
            Value::MessageIntegrity(_) => Attribute::MessageIntegrity,
            Value::ErrorCode(_) => Attribute::ErrorCode,
            Value::Lifetime(_) => Attribute::Lifetime,
        }, value).is_some()
    }

    /// 获取属性
    ///
    /// 从消息中的属性列表获取属性.
    pub fn get_attr(&self, key: &Attribute) -> Option<&Value> {
        self.attributes.get(key)
    }
}
