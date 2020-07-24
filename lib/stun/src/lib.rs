mod codec;
// mod payload;

use anyhow::Result;
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
pub struct STUN {}

impl STUN {
    /// 创建STUN服务器
    ///
    /// 通过给定的地址创建STUN服务器，
    /// 该服务器下层实现为UDP Server.
    pub fn new() -> Self {
        Self {}
    }

    /// 处理stun数据包
    ///
    /// 不做任何处理，直接返回响应.
    pub async fn process(&mut self, buffer: BytesMut, addr: SocketAddr) -> Result<()> {
        let message = codec::decoder(buffer)?;
        // let response = payload::process(addr, message)?;
        Ok(())
    }
}

impl Default for STUN {
    fn default() -> Self {
        Self::new()
    }
}
