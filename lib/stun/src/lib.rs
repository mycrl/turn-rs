//! STUN
//! (Simple Traversal of User Datagram Protocol Through Network Address Translators)
//! 
//! 即简单的用UDP穿透NAT，是个轻量级的协议，是基于UDP的完整的穿透NAT的解决方案，
//! 它允许应用程序发现它们与公共互联网之间存在的NAT和防火墙及其他类型，
//! 它也可以让应用程序确定NAT分配给它们的公网IP地址和端口号，
//! STUN是一种Client/Server的协议，也是一种Request/Response的协议，
//! 默认端口号是3478.
//! 
//! ### 消息头
//! > 所有的STUN消息都包含20个字节的消息头，包括16位的消息类型，16位的消息长度和128位的事务ID.
//! 
//! * 0x0001：捆绑请求
//! * 0x0101：捆绑响应
//! * 0x0111：捆绑错误响应
//! * 0x0002：共享私密请求
//! * 0x0102：共享私密响应
//! * 0x0112：共享私密错误响应
//! 
//! 
//! ### 消息属性
//! > 消息头之后是0或多个属性，每个属性进行TLV编码，包括16位的属性类型、16位的属性长度和变长属性值.
//! 
//! * MAPPED-ADDRESS：MAPPED-ADDRESS属性表示映射过的IP地址和端口。它包括8位的地址族，16位的端口号及长度固定的IP地址.
//! * RESPONSE-ADDRESS：RESPONSE-ADDRESS属性表示响应的目的地址.
//! * CHASNGE-REQUEST：客户使用32位的CHANGE-REQUEST属性来请求服务器使用不同的地址或端口号来发送响应.
//! * SOURCE-ADDRESS：SOURCE-ADDRESS属性出现在捆绑响应中，它表示服务器发送响应的源IP地址和端口.
//! * CHANGED-ADDRESS：如果捆绑请求的CHANGE-REQUEST属性中的“改变IP”和“改变端口”标志设置了，则CHANGED-ADDRESS属性表示响应发出的IP地址和端口号.
//! * USERNAME：USERNAME属性用于消息的完整性检查，用于消息完整性检查中标识共享私密。USERNAME通常出现在共享私密响应中，与PASSWORD一起。当使用消息完整性检查时，可有选择地出现在捆绑请求中.
//! * PASSWORD：PASSWORD属性用在共享私密响应中，与USERNAME一起。PASSWORD的值是变长的，用作共享私密，它的长度必须是4字节的倍数，以保证属性与边界对齐.
//! * MESSAGE-INTEGRITY：MESSAGE-INTEGRITY属性包含STUN消息的HMAC-SHA1，它可以出现在捆绑请求或捆绑响应中；MESSAGE-INTEGRITY属性必须是任何STUN消息的最后一个属性。它的内容决定了HMAC输入的Key值.
//! * ERROR-CODE：ERROR-CODE属性出现在捆绑错误响应或共享私密错误响应中。它的响应号数值范围从100到699.
//! * UNKNOWN-ATTRIBUTES：UNKNOWN-ATTRIBUTES属性只存在于其ERROR-CODE属性中的响应号为420的捆绑错误响应或共享私密错误响应中.
//! * REFLECTED-FROM：REFLECTED-FROM属性只存在于其对应的捆绑请求包含RESPONSE-ADDRESS属性的捆绑响应中。属性包含请求发出的源IP地址，它的目的是提供跟踪能力，这样STUN就不能被用作DOS攻击的反射器.
//! 
//! ### 具体的ERROR-CODE（响应号）定义：
//! 
//! * 400（错误请求）：请求变形了。客户在修改先前的尝试前不应该重试该请求.
//! * 401（未授权）：捆绑请求没有包含MESSAGE-INTERITY属性.
//! * 420（未知属性）：服务器不认识请求中的强制属性.
//! * 430（过期资格）：捆绑请求没有包含MESSAGE-INTEGRITY属性，但它使用过期的共享私密。客户应该获得新的共享私密并再次重试.
//! * 431（完整性检查失败）：捆绑请求包含MESSAGE-INTEGRITY属性，但HMAC验证失败。这可能是潜在攻击的表现，或者客户端实现错误.
//! * 432（丢失用户名）：捆绑请求包含MESSAGE-INTEGRITY属性，但没有USERNAME属性。完整性检查中两项都必须存在.
//! * 433（使用TLS）：共享私密请求已经通过TLS（Transport Layer Security，即安全传输层协议）发送，但没有在TLS上收到.
//! * 500（服务器错误）：服务器遇到临时错误，客户应该再次尝试.
//! * 600（全局失败）：服务器拒绝完成请求，客户不应该重试.
//! 
//! 属性空间分为可选部分与强制部分，值超过0x7fff的属性是可选的，即客户或服务器即使不认识该属性也能够处理该消息；
//! 值小于或等于0x7fff的属性是强制理解的，即除非理解该属性，否则客户或服务器就不能处理该消息.
//! 

use std::{io::Error, net::SocketAddr};
use tokio::net::{UdpSocket, ToSocketAddrs};
use stun_codec::{Message, MessageEncoder, MessageDecoder, MessageClass};
use stun_codec::rfc5389::{Attribute, attributes::XorMappedAddress};
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
        })
    }

    /// 处理STUN服务器任务
    /// 
    /// TODO: 目前只处理了请求消息，未处理其他消息，
    /// 对于客户端绑定请求，暂时未处理，后续看情况判断是否添加.
    pub async fn process(&mut self) -> Result<(), Error> {
        if let Some((message, addr)) = self.recv_message().await {
            if let Some(response) = self.into_success_message(message, addr) {
                self.socket.send_to(&response, addr).await?;
            }
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
                Ok(Ok(message)) if message.attributes().count() > 0 => Some((message, addr)),
                _ => None
            }, _ => None
        }
    }

    /// 转换确认消息
    /// 
    /// 尝试把STUN消息序列化为字节缓冲区，
    /// 这里为确定客户端请求，将外网地址和端口回复给客户端.
    fn into_success_message(&mut self, message: Message<Attribute>, addr: SocketAddr) -> Option<Bytes> {
        let method = message.method();
        let id = message.transaction_id();
        let class = MessageClass::SuccessResponse;
        let mut response = Message::<Attribute>::new(class, method, id);
        let address = Attribute::XorMappedAddress(XorMappedAddress::new(addr));
        response.add_attribute(address);
        match self.encoder.encode_into_bytes(response) {
            Ok(buffer) => Some(Bytes::from(buffer)),
            _ => None
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
