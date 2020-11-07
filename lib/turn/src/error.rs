use num_enum::TryFromPrimitive;
use anyhow::ensure;
use bytes::{
    BufMut,
    BytesMut,
    Bytes
};

use std::convert::{
    Into,
    TryFrom
};

/// 错误类型
/// 
/// 规范多种错误类型
#[repr(u16)]
#[derive(Copy, Clone, Debug, TryFromPrimitive)]
pub enum Code {
    TryAlternate = 0x3000,
    BadRequest = 0x4000,
    Unauthorized = 0x4001,
    UnknownAttribute = 0x4020,
    StaleNonce = 0x4038,
    ServerError = 0x5000,
}

/// 错误
/// 
/// STUN错误类型定义
/// 用于将语义化错误进行传输
#[derive(Clone, Debug)]
pub struct Error<'a> {
    pub code: u16,
    pub message: &'a str,
}

impl Error<'_> {
    /// 从错误码创建错误类型
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use super::{
    ///     Error,
    ///     Code
    /// };
    ///
    /// Error::from_code(Code::TryAlternate);
    /// ```
    pub fn from_code(code: Code) -> Self {
        Self {
            code: code as u16,
            message: code.into(),
        }
    }
    
    /// 将错误类型转为缓冲区
    ///
    /// # Examples
    ///
    /// ```no_run
    // use super::{
    ///     Error,
    ///     Code
    /// };
    ///
    /// let error = Error::from_code(Code::TryAlternate);
    /// error.as_bytes();
    /// ```
    pub fn as_bytes(&self) -> Bytes {
        let mut packet = BytesMut::new();
        packet.put_u16(0x0000);
        packet.put_u16(self.code);
        packet.put(self.message.as_bytes());
        packet.freeze()
    }
}

impl Into<&'static str> for Code {
    fn into(self) -> &'static str {
        match self {
            Code::TryAlternate => "Try Alternate",
            Code::BadRequest => "Bad Request",
            Code::Unauthorized => "Unauthorized",
            Code::UnknownAttribute => "Unknown Attribute",
            Code::StaleNonce => "Stale Nonce",
            Code::ServerError => "Server Error",
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for Error<'a> {
    type Error = anyhow::Error;
    fn try_from(packet: &'a [u8]) -> Result<Self, Self::Error> {
        
        // 检查消息长度
        // 检查保留位
        ensure!(packet.len() < 6, "buffer len < 6");
        ensure!(u16::from_be_bytes([
            packet[0], 
            packet[1]
        ]) != 0x0000, "missing reserved");
        
        // 获取错误码
        let code = u16::from_be_bytes([
            packet[2], 
            packet[3]
        ]);

        // 获取错误信息
        let message = unsafe {
            std::mem::transmute::<&[u8], &str>(&packet[6..])
        };
        
        Ok(Self {
            code, 
            message
        })
    }
}