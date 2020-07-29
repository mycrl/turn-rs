use anyhow::{Result, anyhow};
use num_enum::TryFromPrimitive;
use bytes::BufMut;

/// 错误类型
#[repr(u16)]
#[derive(Copy, Clone, Debug, TryFromPrimitive)]
pub enum Code {
    TryAlternate = 0x3000,
    BadRequest = 0x4000,
    Unauthorized = 0x4001,
    UnknownAttribute = 0x4020,
    StaleNonce = 0x4038,
    ServerError = 0x5000
}

/// error code.
#[derive(Clone, Debug)]
pub struct Error {
    pub code: u16,
    pub message: String,
}

impl Error {
    /// 创建错误消息
    /// 
    /// 只需要指定错误码，
    /// 函数内部将自动组装错误消息.
    pub fn new(code: Code) -> Self {
        Self {
            code: code as u16,
            message: (match code {
                Code::TryAlternate => "Try Alternate",
                Code::BadRequest => "Bad Request",
                Code::Unauthorized => "Unauthorized",
                Code::UnknownAttribute => "Unknown Attribute",
                Code::StaleNonce => "Stale Nonce",
                Code::ServerError => "Server Error"
            }).to_string()
        }
    }

    /// 缓冲区解码
    /// 
    /// 将缓冲区转换为错误类型.
    pub fn from(mut buffer: Vec<u8>) -> Result<Self> {
        if buffer.len() < 6 { return Err(anyhow!("buffer len < 6")) }
        let reserved = u16::from_be_bytes([buffer[0], buffer[1]]);
        if reserved != 0x0000 { return Err(anyhow!("missing reserved")) }
        let code = u16::from_be_bytes([buffer[2], buffer[3]]);
        let message = String::from_utf8(buffer.split_off(6))?;
        Ok(Self {
            code,
            message
        })
    }

    /// 转码缓冲区
    /// 
    /// 将错误类型转换为缓冲区.
    pub fn into_bytes(self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.put_u16(0x0000);
        buffer.put_u16(self.code);
        buffer.extend_from_slice(self.message.as_bytes());
        buffer
    }
}
