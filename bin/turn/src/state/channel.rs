use tokio::time::Instant;
use super::Addr;
use std::iter::{
    IntoIterator,
    Iterator
};

/// channels iterator.
pub struct Iter {
    index: usize,
    inner: Channel
}

/// Peer channels.
/// 
/// A channel binding consists of:
/// 
/// *  a channel number;
/// 
/// *  a transport address (of the peer); and
/// 
/// *  A time-to-expiry timer.
/// 
///  Within the context of an allocation, a channel binding is uniquely
/// identified either by the channel number or by the peer's transport
/// address.  Thus, the same channel cannot be bound to two different
/// transport addresses, nor can the same transport address be bound to
/// two different channels.
/// 
/// A channel binding lasts for 10 minutes unless refreshed.  Refreshing
/// the binding (by the server receiving a ChannelBind request rebinding
/// the channel to the same peer) resets the time-to-expiry timer back to
/// 10 minutes.
/// 
/// When the channel binding expires, the channel becomes unbound.  Once
/// unbound, the channel number can be bound to a different transport
/// address, and the transport address can be bound to a different
/// channel number.  To prevent race conditions, the client MUST wait 5
/// minutes after the channel binding expires before attempting to bind
/// the channel number to a different transport address or the transport
/// address to a different channel number.
/// 
/// When binding a channel to a peer, the client SHOULD be prepared to
/// receive ChannelData messages on the channel from the server as soon
/// as it has sent the ChannelBind request.  Over UDP, it is possible for
/// the client to receive ChannelData messages from the server before it
/// receives a ChannelBind success response.
/// 
/// In the other direction, the client MAY elect to send ChannelData
/// messages before receiving the ChannelBind success response.  Doing
/// so, however, runs the risk of having the ChannelData messages dropped
/// by the server if the ChannelBind request does not succeed for some
/// reason (e.g., packet lost if the request is sent over UDP or the
/// server being unable to fulfill the request).  A client that wishes to
/// be safe should either queue the data or use Send indications until
/// the channel binding is confirmed.
pub struct Channel {
    timer: Instant,
    bond: [Option<Addr>; 2],
}

impl Channel {
    pub fn new(a: &Addr) -> Self {
        Self {
            bond: [Some(a.clone()), None],
            timer: Instant::now(),
        }
    }
    
    /// whether to include the current socketaddr.
    /// 
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let channel = Channel::new(&addr);
    /// // channel.includes(&addr)
    /// ```
    pub fn includes(&self, a: &Addr) -> bool {
        self.bond.contains(&Some(a.clone()))
    }

    /// wether the peer addr has been established.
    /// 
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let channel = Channel::new(&addr);
    /// // channel.is_half(&addr)
    /// ```
    pub fn is_half(&self) -> bool {
        self.bond.contains(&None)
    }

    /// update half addr.
    /// 
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let mut channel = Channel::new(&addr);
    /// // channel.up(&peer)
    /// ```
    pub fn up(&mut self, a: &Addr) {
        self.bond[1] = Some(a.clone())
    }

    /// refresh channel lifetime.
    /// 
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let mut channel = Channel::new(&addr);
    /// // channel.refresh()
    /// ```
    pub fn refresh(&mut self) {
        self.timer = Instant::now();
    }
    
    /// whether the channel lifetime has ended.
    /// 
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let mut channel = Channel::new(&addr);
    /// // channel.is_death(600)
    /// ```
    #[rustfmt::skip]
    pub fn is_death(&self) -> bool {
        self.timer.elapsed().as_secs() >= 600
    }
}

impl Iterator for Iter {
    type Item = Addr;
    /// Iterator for channels.
    /// 
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let iter = Iter {
    ///     inner: Channel::new(&addr),
    ///     index: 0,
    /// };
    ///
    /// // iter.next()
    /// ```
    #[rustfmt::skip]
    fn next(&mut self) -> Option<Self::Item> {
        let item = match self.index < 2 {
            true => self.inner.bond[self.index].clone(),
            false => None
        };
        
        self.index += 1;
        item
    }
}

impl IntoIterator for Channel {
    type Item = Addr;
    type IntoIter = Iter;
    /// Into iterator for channels.
    /// 
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let channel = Channel::new(&addr);
    /// let iter = channel.into_iter();
    /// // iter.next()
    /// ```
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            inner: self,
            index: 0
        }
    }
}
