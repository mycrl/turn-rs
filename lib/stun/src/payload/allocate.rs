use crate::codec::{Flag, Message, util};
use crate::codec::attribute::{Attribute, Code};
use crate::codec::error;
use std::net::SocketAddr;

/// 返回错误响应
/// 
/// 返回固定认证错误响应.
fn reject(realm: &String, message: Message) -> Message {
    let mut response = Message::new(Flag::AllocateErrRes, message.transaction);
    response.add_attr(Attribute::ErrorCode(error::Error::new(error::Code::Unauthorized)));
    response.add_attr(Attribute::Nonce(util::rand_string(16)));
    response.add_attr(Attribute::Software(String::from("None")));
    response.add_attr(Attribute::Realm(realm.clone()));
    response
}

/// 分配成功
fn resolve(local: SocketAddr, source: SocketAddr, message: Message) -> Message {
    let mut response = Message::new(Flag::AllocateRes, message.transaction);
    response.add_attr(Attribute::Software(String::from("None")));
    response.add_attr(Attribute::XorMappedAddress(local));
    response.add_attr(Attribute::XorMappedAddress(source));
    response.add_attr(Attribute::Lifetime(600));
    response
}

/// 处理分配请求
/// 
/// 验证认证权限，
/// 如认证错误则返回错误响应.
#[rustfmt::skip]
pub fn handle(local: SocketAddr, source: SocketAddr, realm: &String, message: Message) -> Message {
    let username = message.get_attr(&Code::UserName);
    let source_realm = message.get_attr(&Code::Realm);
    let integrity = message.get_attr(&Code::MessageIntegrity);

    // 检查属性的完整性
    if let None = username { return reject(realm, message) }
    if let None = source_realm { return reject(realm, message) }
    if let None = integrity { return reject(realm, message) }

    // TODO: 暂时不检查，
    // 这个地方需要RPC到core服务获取是否通过认证.
    resolve(local, source, message)
}
