mod binding;
mod allocate;

use super::codec::{Flag, Message};
use std::net::SocketAddr;

/// 处理请求
///
/// 根据不同的消息类型调用不同的处理函数.
#[rustfmt::skip]
pub fn process(local: SocketAddr, source: SocketAddr, realm: &String, message: Message) -> Option<Message> {
    match message.flag {
        Flag::BindingReq => Some(binding::handle(local, source, message)),
        Flag::AllocateReq => Some(allocate::handle(local, source, realm, message)),
        _ => None,
    }
}
