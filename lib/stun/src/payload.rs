use super::codec::{Attribute, Attributes, Flag, Message};
use std::net::SocketAddr;

/// 处理请求
///
/// 根据不同的消息类型调用不同的处理函数.
#[rustfmt::skip]
pub fn process(local: SocketAddr, source: SocketAddr, message: Message) -> Option<Message> {
    match message.flag {
        Flag::BindingReq => Some(binding_request(local, source, message)),
        Flag::AllocateReq => Some(allocate_request(message)),
        _ => None,
    }
}

/// 处理绑定请求
///
/// 注意：这个地方为了降低复杂度，并不会对请求的来源
/// 做任何检查，对于任何绑定请求都直接返回NAT响应.
#[rustfmt::skip]
fn binding_request(local: SocketAddr, source: SocketAddr, message: Message) -> Message {
    let mut message = Message::new(Flag::BindingRes, message.transaction);
    message.add_attr(Attributes::XorMappedAddress, Attribute::XorMappedAddress(source));
    message.add_attr(Attributes::MappedAddress, Attribute::MappedAddress(source));
    message.add_attr(Attributes::ResponseOrigin, Attribute::ResponseOrigin(local));
    message.add_attr(Attributes::Software, Attribute::Software("None".to_string()));
    message
}

/// 处理分配请求
fn allocate_request(message: Message) -> Message {
    let username = message.get_attr(&Attributes::UserName);
    let nonce = message.get_attr(&Attributes::Nonce);
    let integrity = message.get_attr(&Attributes::MessageIntegrity);
    
}
