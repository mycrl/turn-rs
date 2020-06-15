use super::{Event, Rx, Tx};
use bytes::BytesMut;
use std::net::SocketAddr;
use std::{io::Error, sync::Arc};
use std::collections::{HashMap, HashSet};
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpStream};
use transport::{Flag, Payload, Transport};

// type
type Frame = HashMap<String, Arc<Payload>>;

/// 数据搬运
///
/// 用于处理和交换中心之间的通讯，
/// 获取流数据和反馈事件.
///
/// TODO: 单路TCP负载能力有限，
/// 计划使用多路合并来提高传输能力;
pub struct Porter {
    peer: HashMap<String, Vec<Tx>>,
    channel: HashSet<String>,
    transport: Transport,
    video_frame: Frame,
    audio_frame: Frame,
    stream: TcpStream,
    receiver: Rx,
    frame: Frame
}

impl Porter {
    /// 创建数据搬运实例
    ///
    /// 通过指定远程交换中心地址和传入一个读取管道来完成创建，
    /// 外部通过管道向这个模块传递一些基础事件.
    pub async fn new(addr: SocketAddr, receiver: Rx) -> Result<Self, Error> {
        Ok(Self {
            receiver,
            peer: HashMap::new(),
            frame: HashMap::new(),
            channel: HashSet::new(),
            transport: Transport::new(),
            video_frame: HashMap::new(),
            audio_frame: HashMap::new(),
            stream: TcpStream::connect(addr).await?,
        })
    }

    /// 处理远程订阅
    ///
    /// 将订阅事件发送到交换中心，
    /// 通知这个实例已经订阅了这个频道.
    /// 这里需要注意的是，如果已经订阅的频道，
    /// 这个地方将跳过，不需要重复订阅.
    #[rustfmt::skip]
    async fn peer_subscribe(&mut self, name: String) -> Result<(), Error> {
        if self.channel.contains(&name) { return Ok(()); }
        self.channel.insert(name.clone());
        self.stream.write_all(&Transport::encoder(Transport::packet(Payload {
            name,
            timestamp: 0,
            data: BytesMut::new(),
        }), Flag::Pull)).await?;
        self.stream.flush().await?;
        Ok(())
    }

    /// 订阅频道
    ///
    /// 将外部可写管道添加到频道列表中，
    /// 将管道和频道对应绑定.
    async fn subscribe(&mut self, name: String, sender: Tx) -> Result<(), Error> {
        self.peer_subscribe(name.clone()).await?;

        // 发送媒体信息
        // FLV的特殊处理，
        // FLV需要这个信息完成播放.
        if let Some(payload) = self.frame.get(&name) {
            let event = Event::Bytes(Flag::Frame, payload.clone());
            sender.send(event).map_err(drop).unwrap();
        }

        // 发送首帧视频
        // FLV的特殊处理，
        // FLV头帧视频需要H264的配置信息.
        if let Some(payload) = self.video_frame.get(&name) {
            let event = Event::Bytes(Flag::Video, payload.clone());
            sender.send(event).map_err(drop).unwrap();
        }

        // 发送首帧音频
        // FLV的特殊处理，
        // FLV头帧音频需要H264的配置信息.
        if let Some(payload) = self.audio_frame.get(&name) {
            let event = Event::Bytes(Flag::Audio, payload.clone());
            sender.send(event).map_err(drop).unwrap();
        }

        // 将客户端和频道绑定，
        // 方便后续频道的操作直接对应到
        // 客户端，失效时删除即可.
        self.peer.entry(name)
            .or_insert_with(Vec::new)
            .push(sender);
        Ok(())
    }

    /// 处理数据负载
    ///
    /// 将数据负载发送给每个订阅了此频道的管道,
    /// 如果发送失败，这个地方目前当失效处理，
    /// 直接从订阅列表中删除这个管道.
    #[rustfmt::skip]
    fn process_payload(&mut self, flag: Flag, payload: Arc<Payload>) {
        if let Some(peer) = self.peer.get_mut(&payload.name) {
            let mut failure = Vec::new();

            // 处理掉一部分内部
            // 需要管理的消息
            match flag {
                
                // 音视频媒体信息
                // 写入暂存区待后续使用
                Flag::Frame => {
                    self.frame.entry(payload.name.clone())
                        .or_insert_with(|| payload.clone());
                },

                // 视频帧
                // 检查是否存在首个视频帧
                // 如果不存在则缓存首个帧
                // TODO: 此处主要是为了解决FLV
                // 头帧要求的问题
                Flag::Video => {
                    self.video_frame.entry(payload.name.clone())
                        .or_insert_with(|| payload.clone());
                },

                // 音频帧
                // 检查是否存在首个音频帧
                // 如果不存在则缓存首个帧
                // TODO: 此处主要是为了解决FLV
                // 头帧要求的问题
                Flag::Audio => {
                    self.audio_frame.entry(payload.name.clone())
                        .or_insert_with(|| payload.clone());
                },

                // 其他不处理
                _ => ()
            }

            // 遍历所有的客户端，
            // 将消息路由到相应的客户端.
            for (index, tx) in peer.iter().enumerate() {
                if tx.send(Event::Bytes(flag, payload.clone())).is_err() {
                    failure.push(index);
                }
            }

            // 删除失效的管道
            // 因为这里没法确定管道是因为
            // 什么原因也失效，也没必要知道，
            // 直接删除掉无法工作的管道即可.
            for index in failure {
                peer.remove(index);
            }             
        }
    }

    /// 处理读取管道
    ///
    /// 处理外部传入的相关事件，
    /// 处理到内部，比如订阅频道.
    async fn poll_receiver(&mut self) -> Result<(), Error> {
        while let Some(Event::Subscribe(name, sender)) = self.receiver.recv().await {
            self.subscribe(name, sender).await?;
        }

        Ok(())
    }

    /// 处理TcpSocket数据
    ///
    /// 这里将数据从TcpSocket中读取处理，
    /// 并解码数据，直到拆分成单个负载，
    /// 然后再进行相应的处理.
    #[rustfmt::skip]
    async fn poll_socket(&mut self) -> Result<(), Error> {
        let mut receiver = [0u8; 2048];
        let size = self.stream.read(&mut receiver).await?;
        let chunk = BytesMut::from(&receiver[0..size]);
        if let Some(result) = self.transport.decoder(chunk) {
            for (flag, message) in result {
                if let Ok(payload) = Transport::parse(message) {
                    self.process_payload(flag, Arc::new(payload));
                }
            }
        }

        Ok(())
    }

    /// 顺序处理多个任务
    ///
    /// 处理外部的事件通知，
    /// 处理内部TcpSocket数据.
    pub async fn process(&mut self) -> Result<(), Error> {
        self.poll_receiver().await?;
        self.poll_socket().await?;
        Ok(())
    }
}
