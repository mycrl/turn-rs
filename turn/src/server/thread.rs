use tokio::net::UdpSocket;
use bytes::BytesMut;
use std::{
    net::SocketAddr, 
    sync::Arc
};

use crate::{
    proto::Proto,
    config::Conf,
    state::State
};
pub struct ThreadLocal {
    pub state: Arc<State>,
    pub conf: Arc<Conf>,
}

/// server thread worker.
pub struct Thread {
    inner: Arc<UdpSocket>,
    writer: BytesMut,
    reader: Vec<u8>,
    proto: Proto,
}

impl Thread {
    #[rustfmt::skip]
    pub fn builder(local: ThreadLocal, socket: &Arc<UdpSocket>) -> Self {
        Self {
            writer: BytesMut::with_capacity(local.conf.buffer),
            reader: vec![0u8; local.conf.buffer],
            proto: Proto::builder(local),
            inner: socket.clone(),
        }
    }
    
    /// thread poll.
    /// 
    /// read the data packet from the UDP socket and hand 
    /// it to the proto for processing, and send the processed 
    /// data packet to the specified address.
    #[rustfmt::skip]
    pub async fn poll(&mut self) {
        let data = match self.read().await {
            Some((s, a)) => (s, a),
            None => return
        };
        
        let a = data.1;
        let write_buf = &mut self.writer;
        let read_buf = &self.reader[..data.0];
        if let Ok(Some((b, p))) = self.proto.handler(read_buf, write_buf, a).await {
            Self::send(&self.inner, b, p.as_ref()).await
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

impl Clone for ThreadLocal {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            conf: self.conf.clone()
        }
    }
}
