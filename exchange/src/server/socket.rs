use crate::router::{Event, Rx, Tx};
use bytes::BytesMut;
use futures::prelude::*;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
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

    /// 从TcpSocket读取数据
    ///
    /// 单次最大从缓冲区获取2048字节，
    /// 并转换为BytesMut返回.
    ///
    /// TODO: 目前存在重复申请缓冲区的情况，有优化空间；
    #[rustfmt::skip]
    fn read<'b>(&mut self, ctx: &mut Context<'b>) -> Option<BytesMut> {
        let mut receiver = [0u8; 2048];
        match Pin::new(&mut self.socket).poll_read(ctx, &mut receiver) {
            Poll::Ready(Ok(s)) if s > 0 => Some(BytesMut::from(&receiver[0..s])),
            _ => None,
        }
    }

    /// 发送数据到TcpSocket
    ///
    /// 如果出现未完全写入的情况，
    /// 这里将重复重试，直到写入完成.
    #[rustfmt::skip]
    fn send<'b>(&mut self, ctx: &mut Context<'b>, data: &[u8]) -> Result<(), Error> {
        let mut offset: usize = 0;
        let length = data.len();
        loop {
            match Pin::new(&mut self.socket).poll_write(ctx, &data) {
                Poll::Ready(Err(e)) => { return Err(Error::new(ErrorKind::NotConnected, e)); }, 
                Poll::Ready(Ok(s)) => match offset + s >= length {
                    false => { offset += s; },
                    true => { break; }
                }, _ => (),
            }
        }

        Ok(())
    }

    /// 刷新缓冲区并将Tcp数据推送到远端
    ///
    /// 重复尝试刷新，
    /// 直到数据完全发送到对端.
    #[rustfmt::skip]
    fn flush<'b>(&mut self, ctx: &mut Context<'b>) -> Result<(), Error> {
        loop {
            match Pin::new(&mut self.socket).poll_flush(ctx) {
                Poll::Ready(Err(e)) => { return Err(Error::new(ErrorKind::NotConnected, e)); },
                Poll::Ready(Ok(_)) => { break; },
                _ => (),
            }
        }

        Ok(())
    }

    /// 处理TcpSocket数据
    ///
    /// 这里将数据从TcpSocket中读取处理，
    /// 并解码数据，将消息通过管道传递到核心路由.
    fn process_socket<'b>(&mut self, ctx: &mut Context<'b>) -> Result<(), Error> {
        while let Some(chunk) = self.read(ctx) {
            if let Some(result) = self.transport.decoder(chunk) {
                for (flag, message) in result {
                    let event = Event::Bytes(self.addr.clone(), flag, message);
                    if let Err(e) = self.sender.send(event) {
                        return Err(Error::new(ErrorKind::BrokenPipe, e.to_string()))
                    }
                }
            }
        }

        Ok(())
    }

    /// 处理外部事件
    ///
    /// 处理核心路由传递过来的事件消息，
    /// 目前只处理发布事件，因为单个socket对外只做发布.
    fn process_evevt<'b>(&mut self, ctx: &mut Context<'b>) -> Result<(), Error> {
        while let Poll::Ready(Some(event)) = Pin::new(&mut self.receiver).poll_next(ctx) {
            if let Event::Release(data) = event {
                self.send(ctx, &data)?;
                self.flush(ctx)?;
            }
        }

        Ok(())
    }

    /// 顺序处理多个任务
    ///
    /// 处理外部的事件通知，
    /// 处理内部TcpSocket数据.
    fn process<'b>(&mut self, ctx: &mut Context<'b>) -> Result<(), Error> {
        self.process_socket(ctx)?;
        self.process_evevt(ctx)?;
        Ok(())
    }
}

impl Future for Socket {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match self.get_mut().process(ctx) {
            Ok(_) => Poll::Pending,
            Err(_) => Poll::Ready(Ok(()))
        }
    }
}
