use super::util;
use anyhow::ensure;
use num_enum::TryFromPrimitive;
use bytes::{
    BufMut, 
    BytesMut
};

use std::cmp::{
    Eq, 
    PartialEq
};

use std::convert::{
    Into, 
    TryFrom
};

/// 错误类型
#[repr(u16)]
#[derive(TryFromPrimitive)]
#[derive(PartialEq, Eq)]
#[derive(Copy, Clone, Debug)]
pub enum ErrKind {
    TryAlternate = 0x0300,
    BadRequest = 0x0400,
    Unauthorized = 0x0401,
    Forbidden = 0x0403,
    RequestTimedout = 0x0408,
    UnknownAttribute = 0x0420,
    AllocationMismatch = 0x0437,
    StaleNonce = 0x0438,
    AddressFamilyNotSupported = 0x0440,
    WrongCredentials = 0x0441,
    UnsupportedTransportAddress = 0x0442,
    AllocationQuotaReached = 0x0486,
    ServerError = 0x0500,
    InsufficientCapacity = 0x0508,
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
    /// # Example
    ///
    /// ```no_run
    /// use super::*;
    ///
    /// Error::from(ErrKind::TryAlternate);
    /// ```
    pub fn from(code: ErrKind) -> Self {
        Self {
            code: code as u16,
            message: code.into(),
        }
    }

    /// 将错误类型转为缓冲区
    ///
    /// # Unit Test
    ///
    /// ```test(as_bytes)
    /// use super::*;
    /// use bytes::BytesMut;
    ///
    /// let buffer = [
    ///     0x00u8, 0x00, 0x03, 0x00,
    ///     0x54, 0x72, 0x79, 0x20,
    ///     0x41, 0x6c, 0x74, 0x65,
    ///     0x72, 0x6e, 0x61, 0x74,
    ///     0x65
    /// ];
    ///
    /// let mut buf = BytesMut::with_capacity(1280);
    /// let error = Error::from(ErrKind::TryAlternate);
    /// error.as_bytes(&mut buf);
    /// assert_eq!(&buf[..], &buffer);
    /// ```
    pub fn as_bytes(&self, buf: &mut BytesMut) {
        buf.put_u16(0x0000);
        buf.put_u16(self.code);
        buf.put(self.message.as_bytes());
    }
}

impl<'a> TryFrom<&'a [u8]> for Error<'a> {
    type Error = anyhow::Error;
    #[rustfmt::skip]
    fn try_from(packet: &'a [u8]) -> Result<Self, Self::Error> {
        ensure!(packet.len() < 6, "buffer len < 6");
        ensure!(util::as_u16(&packet[..2]) != 0x0000, "missing reserved");
        Ok(Self { 
            code: util::as_u16(&packet[2..4]),
            message: std::str::from_utf8(&packet[6..])?,
        })
    }
}

impl Into<&'static str> for ErrKind {
    fn into(self) -> &'static str {
        match self {
            Self::TryAlternate => "Try Alternate",
            Self::BadRequest => "Bad Request",
            Self::Unauthorized => "Unauthorized",
            Self::Forbidden => "Forbidden",
            Self::RequestTimedout => "Request Timed out",
            Self::UnknownAttribute => "Unknown Attribute",
            Self::AllocationMismatch => "Allocation Mismatch",
            Self::StaleNonce => "Stale Nonce",
            Self::AddressFamilyNotSupported => "Address Family not Supported",
            Self::WrongCredentials => "Wrong Credentials",
            Self::UnsupportedTransportAddress => "Unsupported Transport Address",
            Self::AllocationQuotaReached => "Allocation Quota Reached",
            Self::ServerError => "Server Error",
            Self::InsufficientCapacity => "Insufficient Capacity",
        }
    }
}

impl Eq for Error<'_> {}
impl PartialEq for Error<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code
    }
}