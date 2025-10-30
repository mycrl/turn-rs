#![allow(dead_code)]

#[rustfmt::skip]
pub const BINDING_REQUEST: &[u8] = include_bytes!("./BindingRequest.bin");

#[rustfmt::skip]
pub const BINDING_RESPONSE: &[u8] = include_bytes!("./BindingResponse.bin");

#[rustfmt::skip]
pub const UNAUTHORIZED_ALLOCATE_REQUEST: &[u8] = include_bytes!("./UnauthorizedAllocateRequest.bin");

#[rustfmt::skip]
pub const UNAUTHORIZED_ALLOCATE_RESPONSE: &[u8] = include_bytes!("./UnauthorizedAllocateResponse.bin");

#[rustfmt::skip]
pub const ALLOCATE_REQUEST: &[u8] = include_bytes!("./AllocateRequest.bin");

#[rustfmt::skip]
pub const ALLOCATE_RESPONSE: &[u8] = include_bytes!("./AllocateResponse.bin");

#[rustfmt::skip]
pub const CREATE_PERMISSION_REQUEST: &[u8] = include_bytes!("./CreatePermissionRequest.bin");

#[rustfmt::skip]
pub const CREATE_PERMISSION_RESPONSE: &[u8] = include_bytes!("./CreatePermissionResponse.bin");

#[rustfmt::skip]
pub const CHANNEL_BIND_REQUEST: &[u8] = include_bytes!("./ChannelBindRequest.bin");

#[rustfmt::skip]
pub const CHANNEL_BIND_RESPONSE: &[u8] = include_bytes!("./ChannelBindResponse.bin");

#[rustfmt::skip]
pub const SEND_INDICATION: &[u8] = include_bytes!("./SendIndication.bin");

#[rustfmt::skip]
pub const DATA_INDICATION: &[u8] = include_bytes!("./DataIndication.bin");

#[rustfmt::skip]
pub const REFRESH_REQUEST: &[u8] = include_bytes!("./RefreshRequest.bin");

#[rustfmt::skip]
pub const REFRESH_RESPONSE: &[u8] = include_bytes!("./RefreshResponse.bin");

#[rustfmt::skip]
pub const CHANNEL_DATA: &[u8] = include_bytes!("./ChannelData.bin");
