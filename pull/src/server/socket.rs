#[rustfmt::skip]

use super::{Event, Rx, Tx};
use futures::prelude::*;
use http::StatusCode;
use std::net::TcpStream;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{error::Error, pin::Pin};
use transport::{Flag, Payload};
use flv::{Tag, Flv, Header};
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
    timestamp_offset: u32,
    completed: bool,
    timestamp: u32,
    send_queue: u8,
    receiver: Rx,
}

impl Socket {
    /// 创建WebSocket实例
    ///
    /// 从TcpSocket创建新的WebSocket实例，
    /// 注意：目前这个实例会不拒绝未发布的频道，
    /// 对于未发布的频道，也会一直等待发布.
    pub fn new(stream: TcpStream, sender: Tx) -> Result<Self, Box<dyn Error>> {
        let (local, receiver) = tokio::sync::mpsc::unbounded_channel();
        Ok(Self {
            receiver,
            timestamp: 0,
            send_queue: 0,
            completed: false,
            timestamp_offset: 0,
            socket: Self::accept(stream, sender, local)?,
        })
    }

    /// 打包FLV TAG
    ///
    /// 需要指定不同的tag类型来打包数据,
    /// 并处理时间戳问题.
    #[rustfmt::skip]
    fn packet_tag(&mut self, payload: Arc<Payload>, tag: Tag) -> Message {
        let flv_packet = Flv::encode_tag(&payload.data, tag, self.timestamp);
        Message::Binary(flv_packet.to_vec())
    }

    /// 打包FLV HEADER
    ///
    /// TODO：这是一个固定的头，
    /// 后期可以优化为常量避免重复分配.
    fn packet_header() -> Message {
        let flv_packet = Flv::encode_header(Header::Full);
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

    /// 处理时间戳
    /// 
    /// 主要为了处理出现重传的情况，
    /// 这个时候不能以推流端的时间戳为准，
    /// 应该在实例内部自行处理时间戳的偏移量.
    fn process_timestamp(&mut self, timestamp: u32) {
        self.timestamp += match self.timestamp_offset > timestamp {
            false => match self.timestamp_offset == 0 {
                false => timestamp - self.timestamp_offset,
                true => 0
            }, true => 0
        };
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
    fn packet(&mut self, flag: Flag, payload: Arc<Payload>) -> Result<(), Box<dyn Error>> {
        let timestamp = payload.timestamp;
        let mut result = Vec::new();

        // 计算并记录当前时间戳，
        // 如果出现重传的情况，可以利用
        // 上次的时间戳计算偏移量.
        self.process_timestamp(timestamp);
        self.timestamp_offset = timestamp;
        self.send_queue += 1;

        // 媒体数据
        let packet = match flag {
            Flag::Audio => Some(self.packet_tag(payload, Tag::Audio)),
            Flag::Video => Some(self.packet_tag(payload, Tag::Video)),
            Flag::Frame if !self.completed => Some(self.packet_tag(payload, Tag::Script)), 
            _ => None,
        };

        // 是首次发送
        // 先添加固定头数据
        if !self.completed {
            result.push(Self::packet_header());
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
            self.socket.write_message(message)?;
        }
        
        // 当达到写入一定量消息之后，
        // 清空缓冲区并将所有挂起的消息都处理掉.
        if self.send_queue > 5 {
            self.socket.write_pending()?;
            self.send_queue = 0;
        }

        Ok(())
    }

    /// 尝试处理返回事件
    ///
    /// 注意: 这里只处理部分事件,
    /// 比如媒体数据包事件.
    fn process<'b>(&mut self, ctx: &mut Context<'b>) -> Result<(), Box<dyn Error>> {
        while let Poll::Ready(Some(event)) = Pin::new(&mut self.receiver).poll_next(ctx) {
            if let Event::Bytes(flag, payload) = event {
                self.packet(flag, payload)?;
            }
        }

        Ok(())
    }
}

impl Future for Socket {
    type Output = Result<(), std::io::Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match self.get_mut().process(ctx) {
            Ok(_) => Poll::Pending,
            Err(_) => Poll::Ready(Ok(()))
        }
    }
}
