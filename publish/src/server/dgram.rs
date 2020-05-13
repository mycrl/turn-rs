use super::Rx;
use futures::prelude::*;
use std::pin::Pin;
use std::io::Error;
use std::task::{Context, Poll};
use std::net::{UdpSocket, SocketAddr};

/// Udp实例
/// 
/// 负责对外广播音视频数据和控制信息.
/// 全局的Rtmp音视频消息都通过此模块发送到远端.
pub struct Dgram {
    addr: SocketAddr,
    dgram: UdpSocket,
    receiver: Rx
}

impl Dgram {
    /// 创建Udp实例
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// use dgram::Dgram;
    /// use tokio::sync::mpsc;
    ///
    /// let addr = "0.0.0.0:1936".parse().unwrap();
    /// let (_, receiver) = mpsc::unbounded_channel();
    /// 
    /// Dgram::new(addr, receiver).await.unwrap();
    /// ```
    pub fn new(addr: SocketAddr, receiver: Rx) -> Result<Self, Error> {
        Ok(Self {
            addr,
            receiver,
            dgram: UdpSocket::bind("0.0.0.0:0")?
        })
    }

    /// 发送数据到远端Udp服务器
    /// 
    /// 将Udp数据写入到UdpSocket.
    /// 检查是否写入完成，如果未完全写入，写入剩余部分.
    /// 
    /// TODO: 异常处理未完善, 未处理意外情况，可能会出现死循环; 
    fn send(&mut self, data: &[u8]) {
        let mut offset: usize = 0;
        let length = data.len();
        loop {
            match self.dgram.send_to(data, self.addr) {
                Ok(s) => match &offset + &s >= length {
                    false => { offset += s; },
                    true => { break; }
                }, _ => (),
            }
        }
    }

    /// 尝试处理管道中的Udp数据包
    /// 
    /// 重复尝试读取管道数据，
    /// 如读取到则发送数据包并继续尝试.
    fn process<'b>(&mut self, ctx: &mut Context<'b>) {
        while let Poll::Ready(Some(data)) = Pin::new(&mut self.receiver).poll_next(ctx) {
            self.send(&data);
        }
    }
}

impl Future for Dgram {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().process(ctx);
        Poll::Pending
    }
}
