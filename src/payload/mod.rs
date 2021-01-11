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
///
/// * `Message` TURN结构消息
/// * `ChannelData` 频道数据消息
pub enum Payload<'a> {
    Message(Message<'a>),
    ChannelData(ChannelData<'a>),
}

/// 频道数据
///
/// * `buf` 缓冲区引用
/// * `number` 频道号
pub struct ChannelData<'a> {
    pub buf: &'a [u8],
    pub number: u16,
}

/// 消息
///
/// * `block` 有效块位置偏移  
/// * `buffer` 缓冲区引用
/// * `attributes` 属性列表
/// * `token` 消息交易ID
/// * `kind` 消息类型
#[derive(Debug)]
pub struct Message<'a> {
    attributes: Vec<(AttrKind, Property<'a>)>,
    buffer: &'a [u8],
    token: &'a [u8],
    block: u16,
    pub kind: Kind,
}

impl<'a> Message<'a> {
    /// 依赖旧实例创建新的实例
    ///
    /// 消息内部交易号保持为一致性，
    /// 引用旧消息交易号创建新的消息
    ///
    /// # Unit Test
    ///
    /// ```test(from)
    /// use super::*;
    /// use super::codec::*;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    ///   
    /// let old_message = decode_message(&buffer).unwrap();
    /// let message = Message::from(Kind::BindingResponse, &old_message);
    /// assert_eq!(Kind::BindingResponse, message.kind);
    /// ```
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

    /// 依赖旧实例创建新的实例
    ///
    /// 消息内部交易号保持为一致性，
    /// 引用旧消息交易号创建新的消息
    ///
    /// # Unit Test
    ///
    /// ```test(extends)
    /// use super::*;
    /// use super::codec::*;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    /// 
    /// let old_message = decode_message(&buffer).unwrap();
    /// let message = old_message.extends(Kind::BindingResponse);
    /// 
    /// assert_eq!(Kind::BindingResponse, message.kind);
    /// ```
    pub fn extends(&self, kind: Kind) -> Self {
        Self::from(kind, self)
    }

    /// 添加属性
    ///
    /// 添加属性到消息中的属性列表
    ///
    /// # Unit Test
    ///
    /// ```test(append)
    /// use super::*;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    /// 
    /// let old_message = Message::try_from(&buffer[..]).unwrap();
    /// let mut message = Message::from(Kind::BindingResponse, &old_message);
    /// message.append(Property::UserName("panda"));
    /// assert_eq!(message.get(AttrKind::UserName), Some(&Property::UserName("panda")));
    /// ```
    pub fn append(&mut self, value: Property<'a>) {
        self.attributes.push((value.attr(), value));
    }

    /// 获取属性
    ///
    /// 从消息中的属性列表获取属性
    ///
    /// # Unit Test
    ///
    /// ```test(get)
    /// use super::*;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    /// 
    /// let old_message = Message::try_from(&buffer[..]).unwrap();
    /// let mut message = Message::from(Kind::BindingResponse, &old_message);
    /// message.append(Property::UserName("panda"));
    /// assert_eq!(message.get(AttrKind::UserName), Some(&Property::UserName("panda")));
    /// ```
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
    ///
    /// # Unit Test
    ///
    /// ```test(verify)
    /// use super::*;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x03, 0x00, 0x50, 
    ///     0x21, 0x12, 0xa4, 0x42, 
    ///     0x64, 0x4f, 0x5a, 0x78, 
    ///     0x6a, 0x56, 0x33, 0x62, 
    ///     0x4b, 0x52, 0x33, 0x31, 
    ///     0x00, 0x19, 0x00, 0x04, 
    ///     0x11, 0x00, 0x00, 0x00, 
    ///     0x00, 0x06, 0x00, 0x05, 
    ///     0x70, 0x61, 0x6e, 0x64, 
    ///     0x61, 0x00, 0x00, 0x00, 
    ///     0x00, 0x14, 0x00, 0x09, 
    ///     0x72, 0x61, 0x73, 0x70, 
    ///     0x62, 0x65, 0x72, 0x72, 
    ///     0x79, 0x00, 0x00, 0x00, 
    ///     0x00, 0x15, 0x00, 0x10, 
    ///     0x31, 0x63, 0x31, 0x33, 
    ///     0x64, 0x32, 0x62, 0x32, 
    ///     0x34, 0x35, 0x62, 0x33, 
    ///     0x61, 0x37, 0x33, 0x34, 
    ///     0x00, 0x08, 0x00, 0x14,
    ///     0xd6, 0x78, 0x26, 0x99, 
    ///     0x0e, 0x15, 0x56, 0x15, 
    ///     0xe5, 0xf4, 0x24, 0x74, 
    ///     0xe2, 0x3c, 0x26, 0xc5, 
    ///     0xb1, 0x03, 0xb2, 0x6d
    /// ];
    /// 
    /// let message = Message::try_from(&buffer[..]).unwrap();
    /// let result = message.verify(("panda", "panda", "raspberry")).unwrap();
    /// assert!(result);
    /// ```
    pub fn verify(&self, auth: Auth) -> Result<bool> {
        codec::assert_integrity(self, auth)
    }

    /// 消息编码
    ///
    /// # Unit Test
    ///
    /// ```test(try_into)
    /// use super::*;
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    /// 
    /// let result = [
    ///     0x00u8, 0x01, 0x00, 0x20,
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42,
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b,
    ///     0x00, 0x08, 0x00, 0x14,
    ///     0x45, 0x0e, 0x6e, 0x44,
    ///     0x52, 0x1e, 0xe8, 0xde,
    ///     0x2c, 0xf0, 0xfa, 0xb6,
    ///     0x9c, 0x5c, 0x19, 0x17,
    ///     0x98, 0xc6, 0xd9, 0xde, 
    ///     0x80, 0x28, 0x00, 0x04,
    ///     0xed, 0x41, 0xb6, 0xbe
    /// ];
    /// 
    /// let mut buf = BytesMut::with_capacity(1280);
    /// let message = Message::try_from(&buffer[..]).unwrap();
    /// message.try_into(&mut buf, Some(("panda", "panda", "raspberry"))).unwrap();
    /// assert_eq!(&buf[..], &result);
    /// ```
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