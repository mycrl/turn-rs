use anyhow::Result;
use bytes::BytesMut;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::UdpSocket;

use super::{config::Conf, controls::Controls, hub::Hub, state::State};

/// 线程实例
pub(crate) struct ThreadContext {
    inner: Arc<UdpSocket>,
    writer: BytesMut,
    reader: Vec<u8>,
    hub: Hub,
}

impl ThreadContext {
    pub fn new(s: &Arc<UdpSocket>, f: &Arc<Conf>, c: &Arc<State>, r: &Arc<Controls>) -> Self {
        Self {
            hub: Hub::new(f.clone(), c.clone(), r.clone()),
            writer: BytesMut::with_capacity(f.buffer),
            reader: vec![0u8; f.buffer],
            inner: s.clone(),
        }
    }

    /// 线程循环
    ///
    /// 读取UDP数据包并处理，
    /// 将回写包发送到指定远端

    pub async fn poll(&mut self) {
        if let Some((size, addr)) = self.read().await {
            match self
                .hub
                .process(&self.reader[..size], &mut self.writer, addr)
                .await
            {
                Ok(Some((b, p))) => Self::send(&self.inner, b, p.as_ref()).await,
                Err(e) => log::error!("hub err: {}", e),
                _ => (),
            }
        }
    }

    /// 读取UDP数据包
    ///
    /// 读取并检查是否未空包
    /// TODO: 因为tokio udp已知问题，
    /// 远程主机关闭也会导致读取错误，所以这里
    /// 忽略任何读取错误，这是不得已的处理办法
    async fn read(&mut self) -> Option<(usize, SocketAddr)> {
        match self.inner.recv_from(&mut self.reader[..]).await {
            Ok(r) if r.0 >= 4 => Some(r),
            _ => None,
        }
    }

    /// 发送UDP数据包
    ///
    /// 发送数据包到指定远端
    /// 当发生错误时将直接推出进程
    async fn send(inner: &Arc<UdpSocket>, buf: &[u8], p: &SocketAddr) {
        if let Err(e) = inner.send_to(buf, p).await {
            log::error!("udp io error: {}", e);
            std::process::abort();
        }
    }
}

/// 启动服务器
///
/// 启动UDP服务器并创建线程池

pub async fn run(f: Arc<Conf>, c: Arc<State>, r: Arc<Controls>) -> Result<()> {
    let s = Arc::new(UdpSocket::bind(f.listen).await?);
    let threads = match f.threads {
        None => num_cpus::get(),
        Some(s) => s,
    };

    for _ in 0..threads {
        let mut cx = ThreadContext::new(&s, &f, &c, &r);
        tokio::spawn(async move {
            loop {
                cx.poll().await;
            }
        });
    }

    log::info!("threads size {} is runing", threads);

    log::info!("udp bind to {}", f.listen);

    Ok(())
}
