use super::codec::Message;
use std::net::SocketAddr;

/// 返回响应
///
/// 对于客户端请求返回客户端的对外地址，
/// 注意：因为库实现缺失，这个地方没有实现返回服务端地址的属性.
fn response(
    source: SocketAddr,
    class: MessageClass,
    method: Method,
    transaction: TransactionId,
) -> Result<Message> {
    let mut message = Message::new(class, method, transaction);
    message.add_attribute(Attribute::XorMappedAddress(XorMappedAddress::new(source)));
    message.add_attribute(Attribute::MappedAddress(MappedAddress::new(source)));
    message.add_attribute(Attribute::Software(Software::new("None".to_string())?));
    Ok(message)
}

/// 处理请求
///
/// 处理客户端绑定请求，
/// 注意：这个地方为了降低复杂度，并不会对请求的来源
/// 做任何检查，对于任何绑定请求都直接返回NAT响应.
pub fn process(source: SocketAddr, message: Message) -> Result<Message> {
    let method = Method::new(0x0101)?;
    let class = MessageClass::SuccessResponse;
    let transaction = message.transaction_id();
    Ok(response(source, class, method, transaction)?)
}
