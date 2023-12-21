use super::ports::capacity;

use ahash::AHashMap;

use std::iter::{IntoIterator, Iterator};
use std::{net::SocketAddr, sync::RwLock, time::Instant};

/// channels iterator.
pub struct Iter {
    index: usize,
    inner: Channel,
}

impl Iter {
    pub fn new(channel: Channel) -> Self {
        Self {
            inner: channel,
            index: 0,
        }
    }
}

impl Iterator for Iter {
    type Item = SocketAddr;

    /// Iterator for channels.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::channels::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let mut iter = Iter::new(Channel::new(&addr));
    ///
    /// assert_eq!(iter.next(), Some(addr));
    /// ```
    fn next(&mut self) -> Option<Self::Item> {
        let item = match self.index < 2 {
            true => self.inner.bound[self.index],
            false => None,
        };

        self.index += 1;
        item
    }
}

/// Peer channels.
///
/// A channel binding consists of:
///
/// * a channel number;
///
/// * a transport address (of the peer); and
///
/// * A time-to-expiry timer.
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
    bound: [Option<SocketAddr>; 2],
}

impl Channel {
    pub fn new(a: &SocketAddr) -> Self {
        Self {
            bound: [Some(*a), None],
            timer: Instant::now(),
        }
    }

    /// whether to include the current socketaddr.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::channels::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let channel = Channel::new(&addr);
    /// assert!(channel.includes(&addr));
    /// ```
    pub fn includes(&self, a: &SocketAddr) -> bool {
        self.bound.contains(&Some(*a))
    }

    /// wether the peer addr has been established.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::channels::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let channel = Channel::new(&addr);
    /// assert!(channel.is_half());
    /// ```
    pub fn is_half(&self) -> bool {
        self.bound.contains(&None)
    }

    /// update half addr.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::channels::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let mut channel = Channel::new(&addr);
    ///
    /// channel.up(&peer);
    /// assert!(!channel.is_half());
    /// ```
    pub fn up(&mut self, a: &SocketAddr) {
        self.bound[1] = Some(*a)
    }

    /// refresh channel lifetime.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::channels::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let mut channel = Channel::new(&addr);
    ///
    /// channel.refresh();
    /// assert!(!channel.is_death());
    /// ```
    pub fn refresh(&mut self) {
        self.timer = Instant::now();
    }

    /// whether the channel lifetime has ended.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::channels::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let mut channel = Channel::new(&addr);
    /// // channel.is_death()
    /// ```
    pub fn is_death(&self) -> bool {
        self.timer.elapsed().as_secs() >= 600
    }
}

impl IntoIterator for Channel {
    type IntoIter = Iter;
    type Item = SocketAddr;

    /// Into iterator for channels.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::channels::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let channel = Channel::new(&addr);
    /// let iter = channel.into_iter();
    /// // iter.next()
    /// ```
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            inner: self,
            index: 0,
        }
    }
}

/// channels table.
pub struct Channels {
    map: RwLock<AHashMap<u16, Channel>>,
    bounds: RwLock<AHashMap<(SocketAddr, u16), SocketAddr>>,
}

impl Default for Channels {
    fn default() -> Self {
        Self::new()
    }
}

impl Channels {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(AHashMap::with_capacity(capacity())),
            bounds: RwLock::new(AHashMap::with_capacity(capacity())),
        }
    }

    /// get bound address.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::channels::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let channels = Channels::new();
    ///
    /// channels.insert(&addr, 43159, &peer).unwrap();
    /// channels.insert(&peer, 43160, &addr).unwrap();
    ///
    /// assert_eq!(channels.get_bound(&addr, 43159).unwrap(), peer);
    /// ```
    pub fn get_bound(&self, a: &SocketAddr, c: u16) -> Option<SocketAddr> {
        self.bounds.read().unwrap().get(&(*a, c)).cloned()
    }

    /// insert address for peer address to channel table.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::channels::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let channels = Channels::new();
    ///
    /// channels.insert(&addr, 43159, &peer).unwrap();
    /// channels.insert(&peer, 43160, &addr).unwrap();
    ///
    /// assert_eq!(channels.get_bound(&addr, 43159).unwrap(), peer);
    /// ```
    pub fn insert(&self, a: &SocketAddr, c: u16, p: &SocketAddr) -> Option<()> {
        let mut map = self.map.write().unwrap();
        let mut is_empty = false;

        let channel = map.entry(c).or_insert_with(|| {
            is_empty = true;
            Channel::new(a)
        });

        let is_include = if !is_empty { channel.includes(a) } else { true };
        if !channel.is_half() && !is_include {
            return None;
        }

        if !is_include {
            channel.up(a);
        }

        if !is_empty && is_include {
            channel.refresh();
        }

        self.bounds
            .write()
            .unwrap()
            .entry((*a, c))
            .or_insert_with(|| *p);
        Some(())
    }

    /// remove channel allocate in channel table.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::channels::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let channels = Channels::new();
    ///
    /// channels.insert(&addr, 43159, &peer).unwrap();
    /// channels.insert(&peer, 43160, &addr).unwrap();
    ///
    /// assert!(channels.remove(43159).is_some());
    /// assert!(channels.remove(43160).is_some());
    /// ```
    pub fn remove(&self, c: u16) -> Option<()> {
        let mut bounds = self.bounds.write().unwrap();
        for a in self.map.write().unwrap().remove(&c)? {
            bounds.remove(&(a, c));
        }

        Some(())
    }

    /// get death channels.
    ///
    /// ```
    /// use turn_rs::router::channels::*;
    ///
    /// let channels = Channels::new();
    /// assert_eq!(channels.get_deaths().len(), 0);
    /// ```
    pub fn get_deaths(&self) -> Vec<u16> {
        self.map
            .read()
            .unwrap()
            .iter()
            .filter(|(_, v)| v.is_death())
            .map(|(k, _)| *k)
            .collect::<Vec<u16>>()
    }
}
