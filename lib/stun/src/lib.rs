//! ## Session Traversal Utilities for NAT (STUN)
//! 
//! STUN is intended to be used in the context of one or more NAT
//! traversal solutions.  These solutions are known as "STUN Usages".
//! Each usage describes how STUN is utilized to achieve the NAT
//! traversal solution.  Typically, a usage indicates when STUN messages
//! get sent, which optional attributes to include, what server is used,
//! and what authentication mechanism is to be used.  Interactive
//! Connectivity Establishment (ICE) [RFC8445](https://tools.ietf.org/html/rfc8445) 
//! is one usage of STUN. SIP Outbound [RFC5626](https://tools.ietf.org/html/rfc5626) 
//! is another usage of STUN.  In some cases, a usage will require extensions to STUN.  
//! A STUN extension can be in the form of new methods, attributes, or error response codes. 
//! More information on STUN Usages can be found in 
//! [Section 13](https://tools.ietf.org/html/rfc8489#section-13).
//!
//! ### STUN Message Structure
//!
//! ```bash
//! 0                   1                   2                   3
//! 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |0 0|     STUN Message Type     |         Message Length        |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                         Magic Cookie                          |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                                                               |
//! |                     Transaction ID (96 bits)                  |
//! |                                                               |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```
//! 
//! ### STUN Attributes
//! 
//! ```bash
//! 0                   1                   2                   3
//! 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |         Type                  |            Length             |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                         Value (variable)                ....
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```
//!

pub mod error;
pub mod address;
pub mod attribute;
pub mod codec;
pub mod util;

pub use address::Addr;
pub use attribute::{AttrKind, Property};
pub use error::{ErrKind, Error};

use anyhow::Result;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use bytes::BytesMut;

/// (username, password, realm)
pub type Auth<'a> = (&'a str, &'a str, &'a str);

/// message type.
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

/// stun message payload.
pub enum Payload<'a> {
    /// stun message.
    Message(Message<'a>),
    /// channel data message.
    ChannelData(ChannelData<'a>),
}

/// channel data message.
pub struct ChannelData<'a> {
    /// channnel data bytes.
    pub buf: &'a [u8],
    /// channel number.
    pub number: u16,
}

/// stun message.
#[derive(Debug)]
pub struct Message<'a> {
    /// message type.
    pub kind: Kind,
    /// message transaction id.
    pub token: &'a [u8],
    /// message source bytes.
    buffer: &'a [u8],
    /// message effective block bytes size.
    effective_block: u16,
    /// message attribute list.
    attributes: Vec<(AttrKind, Property<'a>)>,
}

impl<'a> Message<'a> {
    /// rely on old message to create new message.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
    /// use stun::codec::*;
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
            effective_block: 0,
            token: old.token,
            buffer: &[],
            kind,
        }
    }

    /// create new message with current message.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
    /// use stun::codec::*;
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

    /// append attribute.
    ///
    /// append attribute to message attribute list.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
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
        self.attributes.push((value.kind(), value));
    }

    /// get attribute.
    ///
    /// get attribute from message attribute list.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
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

    /// check MessageIntegrity attribute.
    /// 
    /// return whether the `MessageIntegrity` attribute 
    /// contained in the message can pass the check.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
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

    /// try decoder bytes as message.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
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
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer: [u8; 20] = [
    ///     0x00, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    ///         
    /// let message = Message::try_from(&buffer[..]).unwrap();
    /// assert_eq!(message.kind, Kind::BindingRequest);
    /// assert_eq!(message.get(AttrKind::UserName), None);
    /// ```
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        codec::decode_message(buf)
    }
}

impl<'a> TryFrom<&'a [u8]> for ChannelData<'a> {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer: [u8; 4] = [
    ///     0x00, 0x01, 0x00, 0x00
    /// ];
    ///         
    /// let data = ChannelData::try_from(&buffer[..]).unwrap();
    /// assert_eq!(data.number, 1);
    /// ```
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