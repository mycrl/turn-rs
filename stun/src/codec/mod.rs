mod decoder;
mod encoder;
mod channel;

use anyhow::Result;
use std::convert::TryFrom;
use num_enum::TryFromPrimitive;

use super::{
    attribute::AttrKind
};

/// message type.
#[repr(u16)]
#[derive(TryFromPrimitive)]
#[derive(PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    BindingRequest              = 0x0001,
    BindingResponse             = 0x0101,
    BindingError                = 0x0111,
    AllocateRequest             = 0x0003,
    AllocateResponse            = 0x0103,
    AllocateError               = 0x0113,
    CreatePermissionRequest     = 0x0008,
    CreatePermissionResponse    = 0x0108,
    CreatePermissionError       = 0x0118,
    SendIndication              = 0x0016,
    DataIndication              = 0x0017,
    ChannelBindRequest          = 0x0009,
    ChannelBindResponse         = 0x0109,
    ChannelBindError            = 0x0119,
    RefreshRequest              = 0x0004,
    RefreshResponse             = 0x0104,
    RefreshError                = 0x0114,
}
