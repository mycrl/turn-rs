use tokio::time::Instant;
use super::Addr;

/// Peer channels.
pub struct Channel {
    pub timer: Instant,
    pub bond: [Option<Addr>; 2],
}

impl Channel {
    /// create channel from socketaddr.
    /// 
    /// ```no_run
    /// use tokio::time::Instant;
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let mut channel = Channel::new(&addr);
    /// ```
    pub fn new(a: &Addr) -> Self {
        Self {
            bond: [Some(a.clone()), None],
            timer: Instant::now(),
        }
    }
    
    /// whether to include the current socketaddr.
    /// 
    /// ```no_run
    /// use tokio::time::Instant;
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
    /// use tokio::time::Instant;
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
    /// use tokio::time::Instant;
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
    /// use tokio::time::Instant;
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
}
