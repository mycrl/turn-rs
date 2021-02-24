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

pub mod attribute;
pub mod util;
mod message;
mod channel;

use anyhow::Result;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
pub use channel::ChannelData;
pub use message::*;

/// message type.
#[repr(u16)]
#[derive(TryFromPrimitive)]
#[derive(PartialEq, Eq, Hash, Debug)]
pub enum Kind {
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
    Message(MessageReader<'a>),
    /// channel data message.
    ChannelData(ChannelData<'a>),
}

impl<'a> TryFrom<&'a [u8]> for Payload<'a> {
    type Error = anyhow::Error;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        assert!(buf.len() >= 4);
        Ok(match buf[0] >> 4 == 4 {
            true => Self::ChannelData(ChannelData::try_from(buf)?),
            false => Self::Message(MessageReader::try_from(buf)?),
        })
    }
}
