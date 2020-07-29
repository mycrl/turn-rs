use super::{attribute::Attribute, Flag, Message, util, MAGIC_COOKIE};
use anyhow::{Result, anyhow};
use bytes::{Buf, BytesMut};
use std::collections::HashMap;
use std::convert::TryFrom;

/// 解码消息
///
/// 仅支持部分类型的消息，
/// 如果遇到不支持的消息将发生错误.
#[rustfmt::skip]
pub fn decoder(mut buffer: BytesMut) -> Result<Message> {
    if buffer.len() < 20 { return Err(anyhow!("message len < 20")) }
    let mut attributes = HashMap::new();

    // 消息类型
    // 消息长度
    let flag = Flag::try_from(buffer.get_u16())?;
    let size = buffer.get_u16() as usize;

    // 检查固定Cookie
    // 检查长度是否足够
    if buffer.get_u32() != MAGIC_COOKIE { return Err(anyhow!("missing cookie")) }
    if buffer.remaining() < size + 12 { return Err(anyhow!("missing len")) }

    // 获取交易号
    let mut transaction = [0u8; 12];
    buffer.copy_to_slice(&mut transaction);

    loop {
        // 如果长度不够继续完成，
        // 则跳出循环返回所有的字段.
        if buffer.remaining() < 4 {
            break;
        }

        // 获取属性类型
        // 获取属性长度
        let key = buffer.get_u16();
        let size = buffer.get_u16() as usize;
        let psize = util::pad_size(size);

        // 获取属性内容
        let mut value = vec![0u8; size];
        buffer.copy_to_slice(&mut value);

        // 此处为了兼容填充位，将
        // 消耗掉填充位.
        if psize > 0 {
            buffer.advance(psize);
        }

        // 如果是受支持的类型，
        // 和受支持的内容，则写入
        // 到属性列表.
        if let Ok(dyn_attribute) = Attribute::try_from(key) {
            if let Ok(attribute) = dyn_attribute.from(transaction, value) {
                attributes.insert(dyn_attribute, attribute);
            }
        }
    }

    Ok(Message {
        flag,
        transaction,
        attributes,
    })
}
