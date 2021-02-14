use tokio::net::UdpSocket;
use bytes::BytesMut;
use anyhow::Result;
use std::{
    net::SocketAddr, 
    sync::Arc
};

use super::{
    rpc::Rpc,
    hub::Hub,
    config::Conf,
    state::State
};

/// server thread worker.
pub(crate) struct Worker {
    inner: Arc<UdpSocket>,
    writer: BytesMut,
    reader: Vec<u8>,
    hub: Hub,
}

impl Worker {
    #[rustfmt::skip]
    pub fn new(
        s: &Arc<UdpSocket>,
        f: &Arc<Conf>, 
        c: &Arc<State>, 
        r: &Arc<Rpc>
    ) -> Self {
        Self {
            hub: Hub::new(f.clone(), c.clone(), r.clone()),
            writer: BytesMut::with_capacity(f.buffer),
            reader: vec![0u8; f.buffer],
            inner: s.clone(),
        }
    }
    
    /// thread poll.
    /// 
    /// read the data packet from the UDP socket and hand 
    /// it to the hub for processing, and send the processed 
    /// data packet to the specified address.
    #[rustfmt::skip]
    pub async fn poll(&mut self) {
        if let Some((size, addr)) = self.read().await {
            match self.hub.process(&self.reader[..size], &mut self.writer, addr).await {
                Ok(Some((b, p))) => Self::send(&self.inner, b, p.as_ref()).await,
                Err(e) => log::error!("remux err: {}", e),
                _ => (),
            }  
        }
    }

    /// read data from udp socket.
    ///
    /// TODO: because tokio udp has some problems, 
    /// if the remote host is shut down, 
    /// it will cause reading errors, 
    /// so any reading errors are ignored here. 
    /// this is a last resort.
    async fn read(&mut self) -> Option<(usize, SocketAddr)> {
        match self.inner.recv_from(&mut self.reader[..]).await {
            Ok(r) if r.0 >= 4 => Some(r), 
            _ => None
        }
    }
    
    /// send UDP data to specified address.
    ///
    /// TODO: if there is an error, it will not recover.
    async fn send(inner: &Arc<UdpSocket>, buf: &[u8], p: &SocketAddr) {
        if let Err(e) = inner.send_to(buf, p).await {
            log::error!("udp io error: {}", e);
            std::process::abort();
        }
    }
}

/// start UDP server and create thread pool.
#[rustfmt::skip]
pub async fn run(f: Arc<Conf>, c: Arc<State>, r: Arc<Rpc>) -> Result<()> {
    let s = Arc::new(UdpSocket::bind(f.listen).await?); 
    let threads = match f.threads {
        None => num_cpus::get(),
        Some(s) => s
    };
    
    for _ in 0..threads {
        let mut cx = Worker::new(&s, &f, &c, &r);
        tokio::spawn(async move {
            loop { cx.poll().await; }
        });
    }
    
    log::info!(
        "threads size {} is runing", 
        threads
    );
    
    log::info!(
        "udp bind to {}",
        f.listen
    );

    Ok(())
}
