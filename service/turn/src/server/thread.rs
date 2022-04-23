use tokio::net::UdpSocket;
use bytes::BytesMut;
use std::{
    net::SocketAddr,
    mem::transmute,
    sync::Arc
};

use crate::{
    accepter::Accepter,
    router::Router,
    env::Environment,
};

/// thread local context.
pub struct ThreadLocal {
    pub router: Arc<Router>,
    pub conf: Arc<Environment>,
}

/// server thread worker.
pub struct Thread<'a> {
    accepter: Accepter<'a>,
    socket: Arc<UdpSocket>,
    writer: BytesMut,
    reader: Vec<u8>,
}

impl<'a> Thread<'a> {
    #[rustfmt::skip]
    pub fn builder(local: ThreadLocal, socket: &Arc<UdpSocket>) -> Self {
        Self {
            writer: BytesMut::with_capacity(4096),
            accepter: Accepter::builder(local),
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
    /// let thread_local = ThreadLocal {
    ///     state: s,
    ///     conf: c
    /// };
    ///
    /// let s = Arc::new(UdpSocket::bind(c.listen).await?);
    /// tokio::spawn(async move {
    ///     let mut tr = Thread::builder(thread_local, &s);
    ///     loop { tr.poll().await.unwrap() }
    /// });
    /// ```
    #[rustfmt::skip]
    pub async fn poll(&mut self) {
        let (s, a) = match self.read().await {
            Some(x) => x,
            None => return
        };

        let accepter: &mut Accepter<'_> = unsafe {
            transmute(&mut self.accepter)
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
    /// let thread_local = ThreadLocal {
    ///     state: s,
    ///     conf: c
    /// };
    ///
    /// let s = Arc::new(UdpSocket::bind(c.listen).await?);
    /// let mut tr = Thread::builder(thread_local, &s);
    /// // tr.read().await
    /// ```
    async fn read(&mut self) -> Option<(usize, SocketAddr)> {
        match self.socket.recv_from(&mut self.reader[..]).await {
            Ok(r) if r.0 >= 4 => Some(r), 
            _ => None
        }
    }
}

impl Clone for ThreadLocal {
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
            conf: self.conf.clone()
        }
    }
}
