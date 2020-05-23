use bytes::BytesMut;
use futures::prelude::*;
use std::collections::{HashMap, HashSet};
use std::task::{Context, Poll};
use std::{io::Error, pin::Pin, sync::Arc};
use tokio::sync::mpsc;
use transport::{Flag, Transport};

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
    publish: HashMap<String, Arc<String>>,
    frame: HashMap<String, Arc<BytesMut>>,
    pull: HashMap<String, HashSet<Arc<String>>>,
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
            publish: HashMap::new(),
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
            match flag {
                Flag::Publish => { self.publish.insert(channel, name); },
                Flag::Frame => { self.frame.insert(channel, Arc::new(payload.data)); },
                Flag::Pull => { self.pull.entry(channel).or_insert_with(HashSet::new).insert(name); },
                _ => (),
            };

            // 将打包好的消息广播到
            // 所有已订阅的节点.
            self.broadcast(payload.name, flag, data);
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
    fn process<'b>(&mut self, ctx: &mut Context<'b>) {
        while let Poll::Ready(Some(event)) = Pin::new(&mut self.receiver).poll_next(ctx) {
            self.process_event(event);
        }
    }
}

impl Future for Router {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
