use crate::router::{Event, Rx, Tx};
use bytes::BytesMut;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::io::{Error, ErrorKind};
use tokio::net::TcpStream;
use transport::Transport;

/// TCP Socket实例
pub struct Socket {
    transport: Transport,
    socket: TcpStream,
    addr: Arc<String>,
    receiver: Rx,
    sender: Tx,
}

impl Socket {
    /// 创建TCP Socket实例
    ///
    /// 接受一个TcpStream，并指定远端地址和写入管道,
    /// 写入管道用于每个Socket和核心路由之间的事件通信.
    pub fn new(socket: TcpStream, addr: Arc<String>, sender: Tx) -> Self {
        let (peer, receiver) = tokio::sync::mpsc::unbounded_channel();
        sender
            .send(Event::Socket(addr.clone(), peer))
            .map_err(drop)
            .unwrap();
        Self {
            addr,
            socket,
            sender,
            receiver,
            transport: Transport::new(),
        }
    }

    /// 处理TcpSocket数据
    ///
    /// 这里将数据从TcpSocket中读取处理，
    /// 并解码数据，将消息通过管道传递到核心路由.
    async fn poll_socket(&mut self) -> Result<(), Error> {
        let mut receiver = [0u8; 2048];
        let size = self.socket.read(&mut receiver).await?;
        let chunk = BytesMut::from(&receiver[0..size]);
        if let Some(result) = self.transport.decoder(chunk) {
            for (flag, message) in result {
                let event = Event::Bytes(self.addr.clone(), flag, message);
                if let Err(e) = self.sender.send(event) {
                    return Err(Error::new(ErrorKind::BrokenPipe, e.to_string()))
                }
            }
        }

        Ok(())
    }

    /// 处理外部事件
    ///
    /// 处理核心路由传递过来的事件消息，
    /// 目前只处理发布事件，因为单个socket对外只做发布.
    async fn poll_evevt(&mut self) -> Result<(), Error> {
        if let Some(Event::Release(data)) = self.receiver.recv().await {
            self.socket.write_all(&data).await?;
            self.socket.flush().await?;
        }

        Ok(())
    }

    // 合并处理
    // 先处理TCP再处理内部事件，
    // 因为考虑到TCP吞吐和事件传递流程而这样排序.
    pub async fn process(&mut self) -> Result<(), Error> {
        self.poll_socket().await?;
        self.poll_evevt().await?;
        Ok(())
    }
}
