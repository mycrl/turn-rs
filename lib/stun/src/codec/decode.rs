use super::{Flag, Attribute, Attributes};
use std::mem::transmute;
use bytes::{BytesMut, Buf};

/// 解码属性
pub fn decode_attribute(mut buffer: BytesMut) {
    let flag = unsafe { transmute::<u16, Attributes>(buffer.get_u16()) };
    let size = buffer.get_u16() as usize;
    // match flag {
    //     Attributes::UserName => 
    // }
}

/// 解码消息
pub fn decode(mut buffer: BytesMut) {
    assert_eq!(buffer.len() >= 20, true);

    // 消息类型
    // 消息长度
    let flag = unsafe { transmute::<u16, Flag>(buffer.get_u16()) };
    let size = buffer.get_u16();

    // 检查固定Cookie
    // 检查长度是否足够
    assert_eq!(buffer.get_u32(), 0x2112A442);
    assert_eq!(buffer.len() >= size as usize + 12, true);

    // 获取交易号
    let mut transaction = [0u8; 12];
    buffer.copy_to_slice(&mut transaction);
}
