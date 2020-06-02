mod performance;

use futures::prelude::*;
use std::net::SocketAddr;
use std::{thread, time};
use std::net::UdpSocket;
use std::task::{Context, Poll};
use std::{io::Error, pin::Pin, io::ErrorKind};
use transport::{Transport, Payload, Flag};
use performance::Performance;

/// 负载均衡
/// 
/// 客户端实例，客户端负责
/// 上送负载信息到中心控制服务.
pub struct Balance {
    stream: UdpSocket,
    addr: SocketAddr,
}

impl Balance {
    /// 创建负载均衡客户端实例
    /// 
    /// 指定一个远端地址，此实例将会
    /// 定时将数据发送到此远端地址.
    pub fn new(addr: SocketAddr) -> Result<Self, Error> {
        let local = "0.0.0.0:0".parse::<SocketAddr>().unwrap();
        Ok(Self{
            addr,
            stream: UdpSocket::bind(local)?,
        })
    }

    /// 发送数据到TcpSocket
    ///
    /// 如果出现未完全写入的情况，
    /// 这里将重复重试，直到写入完成.
    #[rustfmt::skip]
    fn send(&mut self, data: &[u8]) -> Result<(), Error> {
        let mut offset: usize = 0;
        let length = data.len();
        loop {
            match self.stream.send_to(&data, self.addr) {
                Err(e) => { return Err(Error::new(ErrorKind::NotConnected, e)); }, 
                Ok(s) => match offset + s >= length {
                    false => { offset += s; },
                    true => { break; }
                }
            }
        }

        Ok(())
    }

    /// 发送负载信息
    /// 
    /// 间隔10s通过udp将负载信息
    /// 发送到控制中心.
    fn process(&mut self) -> Result<(), Error> {
        thread::sleep(time::Duration::from_secs(10));
        self.send(&Transport::encoder(Transport::packet(Payload {
            timestamp: 0,
            name: "".to_string(),
            data: Performance::new().as_bytes()
        }), Flag::Avg))?;
        Ok(())
    }
}

impl Future for Balance {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, _: &mut Context) -> Poll<Self::Output> {
        match self.get_mut().process() {
            Ok(_) => Poll::Pending,
            Err(_) => Poll::Ready(Ok(()))
        }
    }
}


/// 启动负载均衡实例
///
/// 实例启动之后就无需进行任何操作，
/// 所有的动作都在实例内部自动完成.
pub fn start(addr: SocketAddr) -> Result<(), Error> {
    tokio::spawn(Balance::new(addr)?);
    Ok(())
}
