use std::sync::Arc;
use bytes::BytesMut;
use std::collections::{HashMap, HashSet};
use transport::{Flag, Transport};
use tokio::sync::mpsc;

/// 事件
pub enum Event {
    Socket(Arc<String>, Tx),
    Bytes(Arc<String>, Flag, BytesMut),
    Release(Arc<BytesMut>),
}

/// 事件传递通道
pub type Rx = mpsc::UnboundedReceiver<Event>;
pub type Tx = mpsc::UnboundedSender<Event>;

/// 核心路由
pub struct Router {
    pull: HashMap<String, HashSet<Arc<String>>>,
    video_frame: HashMap<String, BytesMut>,
    audio_frame: HashMap<String, BytesMut>,
    frame: HashMap<String, BytesMut>,
    socket: HashMap<Arc<String>, Tx>,
    receiver: Rx,
}

impl Router {
    /// 创建路由实例
    ///
    /// 传入一个读取管道，
    /// 用于外部传递事件消息到本实例.
    pub fn new(receiver: Rx) -> Self {
        Self {
            receiver,
            pull: HashMap::new(),
            frame: HashMap::new(),
            socket: HashMap::new(),
            video_frame: HashMap::new(),
            audio_frame: HashMap::new(),
        }
    }

    /// 广播消息
    ///
    /// 指定频道名，将消息打包之
    /// 后传递到订阅了此频道的所有socket.
    fn broadcast(&mut self, name: String, flag: Flag, data: BytesMut) {
        let chunk = Arc::new(Transport::encoder(data, flag));
        if let Some(pull) = self.pull.get_mut(&name) {
            let mut failure = Vec::new();
            for addr in pull.iter() {
                if let Some(tx) = self.socket.get(addr) {
                    if tx.send(Event::Release(chunk.clone())).is_err() {
                        failure.push(addr.clone());
                    }
                }
            }

            // 失效管道处理
            // 如果管道发送不成功，
            // 则作为失效处理，从
            // 队列中删除管道.
            for addr in failure {
                pull.remove(&addr);
                self.socket.remove(&addr);
            }
        }
    }

    /// 处理外部数据事件
    ///
    /// 将数据解包并进行相应的处理，
    /// 比如发布，订阅，媒体标识等.
    #[rustfmt::skip]
    fn process_bytes(&mut self, name: Arc<String>, flag: Flag, data: BytesMut) {
        if let Ok(payload) = Transport::parse(data.clone()) {
            let channel = payload.name.clone();
            let mut message = Vec::new();

            // 处理订阅事件
            // 如果一个客户端已经订阅了该频道并且获得了信息，
            // 则没有必要再次为该客户端推送frame信息.
            if let Flag::Pull = flag {
                match self.pull.get(&channel) {
                    Some(pull) if pull.contains(&name) => (),
                    _ => if let Some(frame) = self.frame.get(&channel) {
                        if let Some(audio) = self.audio_frame.get(&channel) {
                            if let Some(video) = self.video_frame.get(&channel) {
                                message.push((Flag::Frame, frame.clone()));
                                message.push((Flag::Audio, audio.clone()));
                                message.push((Flag::Video, video.clone()));
                            }
                        }
                    }
                }
            }

            // 处理掉一部分内部
            // 需要管理的消息
            match flag {
                
                // 音视频媒体信息
                // 写入暂存区待后续使用
                Flag::Frame => {
                    self.frame.insert(channel, data.clone());
                },

                // 拉流事件
                // 有客户端拉流
                // 将客户端和频道关联起来
                Flag::Pull => {
                    self.pull
                        .entry(channel)
                        .or_insert_with(HashSet::new)
                        .insert(name);
                },

                // 推流事件
                // 如果当前频道出现新的推流
                // 这时候应该将暂存区的缓存全部清空
                // 等待下次再次拿到对应数据的时候填充
                Flag::Publish => {
                    self.frame.remove(&channel);
                    self.audio_frame.remove(&channel);
                    self.video_frame.remove(&channel);
                },

                // 视频帧
                // 检查是否存在首个视频帧
                // 如果不存在则缓存首个帧
                // TODO: 此处主要是为了解决FLV
                // 头帧要求的问题
                Flag::Video => {
                    self.video_frame.entry(channel)
                        .or_insert_with(|| data.clone());
                },

                // 音频帧
                // 检查是否存在首个音频帧
                // 如果不存在则缓存首个帧
                // TODO: 此处主要是为了解决FLV
                // 头帧要求的问题
                Flag::Audio => {
                    self.audio_frame.entry(channel)
                        .or_insert_with(|| data.clone());
                },

                // 其他不处理
                _ => ()
            }

            // 如果是负载信息，
            // 则跳过并不广播给其他客户端，
            // 因为这是一个交换中心自己使用的数据.
            // 拉流事件没必要广播
            match flag {
                Flag::Pull => (),
                Flag::Avg => (),
                _ => message.push((flag, data))
            }

            // 将打包好的消息广播到
            // 所有已订阅的节点.
            for (flag, value) in message {
                let name = payload.name.clone();
                self.broadcast(name, flag, value);
            }
        }
    }

    /// 处理外部事件
    ///
    /// 将socket标记到内部，
    /// 或者处理外部传递的数据.
    #[rustfmt::skip]
    fn process_event(&mut self, event: Event) {
        match event {
            Event::Socket(name, tx) => { self.socket.insert(name, tx); },
            Event::Bytes(name, flag, data) => self.process_bytes(name, flag, data),
            _ => (),
        }
    }

    /// 顺序处理多个任务
    ///
    /// 处理外部的事件通知，
    /// 处理内部TcpSocket数据.
    pub async fn process(&mut self) {
        if let Some(event) = self.receiver.recv().await {
            self.process_event(event);
        }
    }
}
