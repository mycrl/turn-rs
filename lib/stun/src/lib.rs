//! STUN
//! (Simple Traversal of User Datagram Protocol Through Network Address Translators)
//! 
//! 即简单的用UDP穿透NAT，是个轻量级的协议，是基于UDP的完整的穿透NAT的解决方案，
//! 它允许应用程序发现它们与公共互联网之间存在的NAT和防火墙及其他类型，
//! 它也可以让应用程序确定NAT分配给它们的公网IP地址和端口号，
//! STUN是一种Client/Server的协议，也是一种Request/Response的协议，
//! 默认端口号是3478.
//! 

mod payload;

use std::{io::Error, net::SocketAddr};
use tokio::net::{UdpSocket, ToSocketAddrs};
use stun_codec::{Message, MessageEncoder, MessageDecoder};
use stun_codec::rfc5389::Attribute;
use bytecodec::{DecodeExt, EncodeExt};
use bytes::{Bytes, BytesMut};

/// STUN服务器
/// 
/// TODO: 用户认证未实现.
#[derive(Debug)]
pub struct STUN {
    socket: UdpSocket,
    decoder: MessageDecoder<Attribute>,
    encoder: MessageEncoder<Attribute>,
    // payload: Payload
}

impl STUN {
    /// 创建STUN服务器
    /// 
    /// 通过给定的地址创建STUN服务器，
    /// 该服务器下层实现为UDP Server.
    pub async fn new<T: ToSocketAddrs>(addr: T) -> Result<Self, Error> {
        Ok(Self {
            decoder: MessageDecoder::new(),
            encoder: MessageEncoder::new(),
            socket: UdpSocket::bind(addr).await?,
            // payload: Payload::new(),
        })
    }

    /// 处理STUN服务器任务
    pub async fn process(&mut self) -> Result<(), Error> {
        if let Some((message, addr)) = self.recv_message().await {
            // if let Ok(response) = self.payload.process(message, addr) {
            //     if let Ok(buffer) = self.encoder.encode_into_bytes(response) {
            //         self.socket.send_to(&buffer, addr).await?;
            //     }
            // }
        }

        Ok(())
    }

    /// 读取消息
    /// 
    /// 尝试从UDP Server中读取消息，
    /// 并包含消息来源的地址.
    async fn read(&mut self) -> Result<(Bytes, SocketAddr), Error> {
        let mut buffer = [0u8; 2048];
        let (size, addr) = self.socket.recv_from(&mut buffer).await?;
        Ok((BytesMut::from(&buffer[0..size]).freeze(), addr))
    }

    /// 获取STUN消息
    /// 
    /// 尝试从UDP消息中序列化出STUN消息，
    /// 这个地方过滤了不需要处理的消息类型.
    #[rustfmt::skip]
    async fn recv_message(&mut self) -> Option<(Message<Attribute>, SocketAddr)> {
        match self.read().await {
            Ok((buffer, addr)) => match self.decoder.decode_from_bytes(&buffer) {
                Ok(Ok(message)) => Some((message, addr)),
                _ => None
            }, _ => None
        }
    }
}

/// 启动服务器
/// 
/// 通过给定的地址启动STUN服务器.
pub async fn start_server<T: ToSocketAddrs>(addr: T) -> Result<(), Error> {
    let mut stun = STUN::new(addr).await?;
    loop {
        stun.process().await?;
    }
}
