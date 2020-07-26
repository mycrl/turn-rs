use super::codec::{Attribute, Attributes, Flag, Message};
use std::net::SocketAddr;

/// 处理请求
///
/// 根据不同的消息类型调用不同的处理函数.
pub fn process(local: SocketAddr, source: SocketAddr, message: Message) -> Option<Message> {
    Some(match message.flag {
        Flag::BindingRequest => binding_request(local, source, message),
        _ => return None,
    })
}

/// 处理绑定请求
///
/// 注意：这个地方为了降低复杂度，并不会对请求的来源
/// 做任何检查，对于任何绑定请求都直接返回NAT响应.
fn binding_request(local: SocketAddr, source: SocketAddr, message: Message) -> Message {
    let mut message = Message::new(Flag::BindingResponse, message.transaction);
    message.add_attr(
        Attributes::XorMappedAddress,
        Attribute::XorMappedAddress(source),
    );
    message.add_attr(Attributes::MappedAddress, Attribute::MappedAddress(source));
    message.add_attr(Attributes::ResponseOrigin, Attribute::ResponseOrigin(local));
    message.add_attr(
        Attributes::Software,
        Attribute::Software("None".to_string()),
    );
    message
}
