use crate::codec::{Flag, Message, attribute::Attribute};
use std::net::SocketAddr;

/// 处理绑定请求
///
/// 注意：这个地方为了降低复杂度，并不会对请求的来源
/// 做任何检查，对于任何绑定请求都直接返回NAT响应.
#[rustfmt::skip]
pub fn handle(local: SocketAddr, source: SocketAddr, message: Message) -> Message {
    let mut message = Message::new(Flag::BindingRes, message.transaction);
    message.add_attr(Attribute::XorMappedAddress(source));
    message.add_attr(Attribute::MappedAddress(source));
    message.add_attr(Attribute::ResponseOrigin(local));
    message.add_attr(Attribute::Software("None".to_string()));
    message
}
