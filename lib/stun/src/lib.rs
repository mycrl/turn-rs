mod codec;
mod payload;

use bytes::BytesMut;
use std::net::SocketAddr;

/// STUN
/// (Simple Traversal of User Datagram Protocol Through Network Address Translators)
///
/// 即简单的用UDP穿透NAT，是个轻量级的协议，是基于UDP的完整的穿透NAT的解决方案，
/// 它允许应用程序发现它们与公共互联网之间存在的NAT和防火墙及其他类型，
/// 它也可以让应用程序确定NAT分配给它们的公网IP地址和端口号，
/// STUN是一种Client/Server的协议，也是一种Request/Response的协议，
/// 默认端口号是3478.
#[derive(Debug)]
pub struct STUN {
    local: SocketAddr
}

impl STUN {
    /// 创建STUN服务器
    ///
    /// 通过给定的地址创建STUN服务器，
    /// 该服务器下层实现为UDP Server.
    pub fn new(local: SocketAddr) -> Self {
        Self { local }
    }

    /// 处理stun数据包
    ///
    /// 不做任何处理，直接返回响应.
    pub fn process(self, buffer: BytesMut, addr: SocketAddr) -> Option<BytesMut> {
        match codec::decoder(buffer) {
            Ok(message) => match payload::process(self.local, addr, message) {
                Some(response) => Some(codec::encoder(response)),
                _ => None
            }, _ => None
        }
    }
}
