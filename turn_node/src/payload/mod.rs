//! RFC: [rfc](https://tools.ietf.org/html/rfc8656)
//! 只兼容此RFC!

mod error;
mod address;
mod attribute;
mod codec;
mod util;

/// !prelude
pub use address::Addr;
pub use attribute::{AttrKind, Property};
pub use error::{ErrKind, Error};

use anyhow::Result;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use bytes::BytesMut;

pub(crate) type Auth<'a> = (
    &'a str, // username
    &'a str, // password
    &'a str  // realm
);

/// 消息类型
#[repr(u16)]
#[derive(TryFromPrimitive)]
#[derive(PartialEq, Eq, Hash)]
#[derive(Copy, Clone, Debug)]
pub enum Kind {
    Unknown = 0x0000,
    BindingRequest = 0x0001,
    BindingResponse = 0x0101,
    BindingError = 0x0111,
    AllocateRequest = 0x0003,
    AllocateResponse = 0x0103,
    AllocateError = 0x0113,
    CreatePermissionRequest = 0x0008,
    CreatePermissionResponse = 0x0108,
    CreatePermissionError = 0x0118,
    SendIndication = 0x0016,
    DataIndication = 0x0017,
    ChannelBindRequest = 0x0009,
    ChannelBindResponse = 0x0109,
    ChannelBindError = 0x0119,
    RefreshRequest = 0x0004,
    RefreshResponse = 0x0104,
    RefreshError = 0x0114,
}

/// 负载
pub enum Payload<'a> {
    /// TURN消息
    Message(Message<'a>),
    /// 频道数据
    ChannelData(ChannelData<'a>),
}

/// 频道数据 
pub struct ChannelData<'a> {
    /// 缓冲区引用
    pub buf: &'a [u8],
    
    /// 频道号
    pub number: u16,
}

/// 消息
#[derive(Debug)]
pub struct Message<'a> {
    /// 属性列表
    attributes: Vec<(AttrKind, Property<'a>)>,
    
    /// 缓冲区引用
    buffer: &'a [u8],
    
    /// 消息交易ID
    token: &'a [u8],
    
    /// 有效块位置偏移
    block: u16,
    
    /// 消息类型
    pub kind: Kind,
}

impl<'a> Message<'a> {
    /// 依赖旧实例创建新的实例
    ///
    /// 消息内部交易号保持为一致性，
    /// 引用旧消息交易号创建新的消息
    pub fn from(kind: Kind, old: &Self) -> Self {
        assert_ne!(kind, Kind::Unknown);
        Self {
            attributes: Vec::new(),
            token: old.token,
            buffer: &[],
            block: 0,
            kind,
        }
    }

    pub fn extends(&self, kind: Kind) -> Self {
        Self::from(kind, self)
    }

    /// 添加属性
    ///
    /// 添加属性到消息中的属性列表
    pub fn append(&mut self, value: Property<'a>) {
        self.attributes.push((value.attr(), value));
    }

    /// 获取属性
    ///
    /// 从消息中的属性列表获取属性
    pub fn get(&self, key: AttrKind) -> Option<&Property> {
        self.attributes
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, v)| v)
    }

    /// 消息完整性检查
    ///
    /// 检查消息中包含的`消息完整性检查`属性
    /// 是否能通过认证
    pub fn verify(&self, auth: Auth) -> Result<bool> {
        codec::assert_integrity(self, auth)
    }

    /// 消息编码
    pub fn try_into(self, buf: &mut BytesMut, auth: Option<Auth>) -> Result<()> {
        codec::encode_message(self, buf, auth)
    }
}

impl<'a> TryFrom<&'a [u8]> for Message<'a> {
    type Error = anyhow::Error;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        codec::decode_message(buf)
    }
}

impl<'a> TryFrom<&'a [u8]> for ChannelData<'a> {
    type Error = anyhow::Error;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        codec::decode_channel(buf)
    }
}

impl<'a> TryFrom<&'a [u8]> for Payload<'a> {
    type Error = anyhow::Error;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        assert!(buf.len() >= 4);
        Ok(match buf[0] >> 4 == 4 {
            true => Payload::ChannelData(ChannelData::try_from(buf)?),
            false => Payload::Message(Message::try_from(buf)?),
        })
    }
}