#[rustfmt::skip]

use super::{Event, Rx, Tx};
use futures::prelude::*;
use http::StatusCode;
use std::net::TcpStream;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{error::Error, pin::Pin};
use transport::{Flag, Payload};
use tungstenite::{
    handshake::server::Request, 
    handshake::server::Response, 
    protocol::WebSocket,
    server::accept_hdr, 
    Message,
};

/// WebSocket
pub struct Socket {
    socket: WebSocket<TcpStream>,
    completed: bool,
    receiver: Rx,
}

impl Socket {
    /// 从TcpSocket创建新的WebSocket实例
    ///
    /// 注意：目前这个实例会不拒绝未发布的频道，
    /// 对于未发布的频道，也会一直等待发布.
    pub fn new(stream: TcpStream, sender: Tx) -> Result<Self, Box<dyn Error>> {
        let (local, receiver) = tokio::sync::mpsc::unbounded_channel();
        Ok(Self {
            receiver,
            completed: false,
            socket: Self::accept(stream, sender, local)?,
        })
    }

    /// 打包FLV TAG
    ///
    /// 需要指定不同的tag类型来打包数据.
    fn packet_binary_tag(payload: Arc<Payload>, tag: flv::Tag) -> Message {
        let data = &payload.data;
        let timestamp = payload.timestamp;
        let flv_packet = flv::encode_tag(&data, tag, timestamp);
        Message::Binary(flv_packet.to_vec())
    }

    /// 打包FLV HEADER
    ///
    /// TODO：这是一个固定的头，
    /// 后期可以优化为常量避免重复分配.
    fn packet_binary_header() -> Message {
        let flv_packet = flv::encode_header(flv::Header::Full);
        Message::Binary(flv_packet.to_vec())
    }

    /// 尝试接受TcpSocket
    ///
    /// 这里将尝试把TcpSocket转换为WebSocket，
    /// 如果中途出现错误，或者出现其他意外情况，
    /// 这里将中断握手并返回404状态码.
    #[rustfmt::skip]
    fn accept(stream: TcpStream, sender: Tx, local_sender: Tx) ->  Result<WebSocket<TcpStream>, Box<dyn Error>> {
        Ok(accept_hdr(stream, move |req: &Request, mut response: Response| {
            let app_name = req.uri().path().to_string().split_off(1);
            if sender.send(Event::Subscribe(app_name, local_sender)).is_err() {
                *response.status_mut() = StatusCode::NOT_FOUND;
            }

            Ok(response)
        })?)
    }

    /// 将单个负载打包成FLV数据
    ///
    /// 这里将检查是否为第一条数据，
    /// 如果为首次发送数据，则先发送flv头信息，
    ///
    /// 注意：这里如果发送不成功，将关闭当前的websocket，
    /// 并且没有处理关闭的Result，如果出现预想中的极端情况，
    /// 这里可能会有问题.
    #[rustfmt::skip]
    fn packet(&mut self, flag: Flag, payload: Arc<Payload>) {
        let mut result = Vec::new();

        // flv包数据
        let packet = match flag {
            Flag::Audio => Some(Self::packet_binary_tag(payload, flv::Tag::Audio)),
            Flag::Video => Some(Self::packet_binary_tag(payload, flv::Tag::Video)),
            Flag::Frame => Some(Self::packet_binary_tag(payload, flv::Tag::Script)), 
            _ => None,
        };

        // 是首次发送
        // 先添加固定头数据
        if !self.completed {
            result.push(Self::packet_binary_header());
            self.completed = true;
        }

        // 有需要处理的包
        // 添加进待发送区
        if let Some(data) = packet {
            result.push(data);
        }

        // 遍历并发送所有消息
        // 如果发送失败，默认连接失效，
        // 这里将关闭连接.
        for message in result {
            if self.socket.write_message(message).is_err() {
                self.socket.close(None).unwrap();
                self.receiver.close();
            }
        }
    }

    /// 尝试处理返回事件
    ///
    /// 注意: 这里只处理部分事件,
    /// 比如媒体数据包事件.
    fn process<'b>(&mut self, ctx: &mut Context<'b>) {
        while let Poll::Ready(Some(event)) = Pin::new(&mut self.receiver).poll_next(ctx) {
            if let Event::Bytes(flag, payload) = event {
                self.packet(flag, payload);
            }
        }
    }
}

impl Future for Socket {
    type Output = Result<(), std::io::Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
