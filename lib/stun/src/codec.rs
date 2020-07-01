use std::mem::transmute;
use bytes::{BytesMut, Buf};

/// STUN方法
#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum Method {
    Binding = 0x0001,
    BindingSuccess = 0x0101,
    Allocate = 0x0003,
    AllocateError = 0x0113,
    AllocateSuccess = 0x0103,
    CreatePermission = 0x0008,
    CreatePermissionSuccess = 0x0108,
    SendIndication = 0x0016,
    DataIndication = 0x0017,
    ChannelBind = 0x0009,
    ChannelBindSuccess = 0x0109,
    ChannelData = 0x4001
}

/// 编解码器
pub struct Codec {
    method: Method,
    length: u16,
    cookie: u32,
    transaction: [u8; 12]
}

impl Codec {
    /// 解码数据包
    pub fn decode(mut buffer: BytesMut) -> Self {
        let mut transaction = [0; 12];
        let method = unsafe { transmute::<u16, Method>(buffer.get_u16()) };
        let length = buffer.get_u16();
        let cookie = buffer.get_u32();
        buffer.copy_to_slice(&mut transaction);
        Self {
            method,
            length,
            cookie,
            transaction
        }
    }
}