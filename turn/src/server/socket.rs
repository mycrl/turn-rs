use tokio::net::UdpSocket;
use bytes::BytesMut;
use std::{
    net::SocketAddr,
    mem::transmute,
    sync::Arc
};

use crate::{
    processor::Processor,
    router::Router,
    args::Args,
};

/// thread local context.
pub struct SocketLocal {
    pub router: Arc<Router>,
    pub args: Arc<Args>,
}

/// server thread worker.
pub struct Socket<'a> {
    processor: Processor<'a>,
    socket: Arc<UdpSocket>,
    writer: BytesMut,
    reader: Vec<u8>,
}

impl<'a> Socket<'a> {
    #[rustfmt::skip]
    pub fn builder(local: SocketLocal, socket: &Arc<UdpSocket>) -> Self {
        Self {
            writer: BytesMut::with_capacity(4096),
            processor: Processor::builder(local),
            reader: vec![0u8; 4096],
            socket: socket.clone(),
        }
    }
    
    /// thread poll.
    /// 
    /// read the data packet from the UDP socket and hand 
    /// it to the proto for processing, and send the processed 
    /// data packet to the specified address.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let c = env::Environment::generate()?;
    /// let t = broker::Broker::new(&c).await?;
    /// let s = state::State::new(t);
    /// 
    /// let thread_local = SocketLocal {
    ///     state: s,
    ///     conf: c
    /// };
    ///
    /// let s = Arc::new(UdpSocket::bind(c.listen).await?);
    /// tokio::spawn(async move {
    ///     let mut tr = Socket::builder(thread_local, &s);
    ///     loop { tr.poll().await.unwrap() }
    /// });
    /// ```
    #[rustfmt::skip]
    pub async fn poll(&mut self) {
        let (s, a) = match self.read().await {
            Some(x) => x,
            None => return
        };

        let accepter: &mut Processor<'_> = unsafe {
            transmute(&mut self.processor)
        };

        let (b, p) = match accepter.handler(
            &self.reader[..s], 
            &mut self.writer, 
            a
        ).await {
            Ok(Some(x)) => x,
            _ => return
        };

        if let Err(e) = self.socket.send_to(b, p.as_ref()).await {
            log::error!("udp io error: {}", e);
            std::process::abort();
        }
    }

    /// read data from udp socket.
    ///
    /// TODO: because tokio udp has some problems, \
    /// if the remote host is shut down, \
    /// it will cause reading errors, \
    /// so any reading errors are ignored here. \ 
    /// this is a last resort.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let c = env::Environment::generate()?;
    /// let t = broker::Broker::new(&c).await?;
    /// let s = state::State::new(t);
    /// 
    /// let thread_local = SocketLocal {
    ///     state: s,
    ///     conf: c
    /// };
    ///
    /// let s = Arc::new(UdpSocket::bind(c.listen).await?);
    /// let mut tr = Socket::builder(thread_local, &s);
    /// // tr.read().await
    /// ```
    async fn read(&mut self) -> Option<(usize, SocketAddr)> {
        match self.socket.recv_from(&mut self.reader[..]).await {
            Ok(r) if r.0 >= 4 => Some(r), 
            _ => None
        }
    }
}

impl Clone for SocketLocal {
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
            args: self.args.clone()
        }
    }
}
