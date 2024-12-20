use std::{
    hash::Hash,
    net::SocketAddr,
    ops::Range,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread::{self, sleep},
    time::Duration,
};

use ahash::{HashMap, HashMapExt};
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use stun::{util::long_key, Transport};

use crate::Observer;

/// Used to lengthen the timing of the release of a readable lock guard and to
/// provide a more convenient way for external access to the lock's internal
/// data.
pub struct ReadLock<'a, 'b, K, R> {
    key: &'a K,
    lock: RwLockReadGuard<'b, R>,
}

impl<'a, 'b, K, V> ReadLock<'a, 'b, K, HashMap<K, V>>
where
    K: Eq + Hash,
{
    pub fn get_ref(&self) -> Option<&V> {
        self.lock.get(self.key)
    }
}

/// Bit Flag
#[derive(PartialEq, Eq)]
pub enum Bit {
    Low,
    High,
}

/// Random Port
///
/// Recently, awareness has been raised about a number of "blind" attacks
/// (i.e., attacks that can be performed without the need to sniff the
/// packets that correspond to the transport protocol instance to be
/// attacked) that can be performed against the Transmission Control
/// Protocol (TCP) [RFC0793] and similar protocols.  The consequences of
/// these attacks range from throughput reduction to broken connections
/// or data corruption [RFC5927] [RFC4953] [Watson].
///
/// All these attacks rely on the attacker's ability to guess or know the
/// five-tuple (Protocol, Source Address, Source port, Destination
/// Address, Destination Port) that identifies the transport protocol
/// instance to be attacked.
///
/// Services are usually located at fixed, "well-known" ports [IANA] at
/// the host supplying the service (the server).  Client applications
/// connecting to any such service will contact the server by specifying
/// the server IP address and service port number.  The IP address and
/// port number of the client are normally left unspecified by the client
/// application and thus are chosen automatically by the client
/// networking stack.  Ports chosen automatically by the networking stack
/// are known as ephemeral ports [Stevens].
///
/// While the server IP address, the well-known port, and the client IP
/// address may be known by an attacker, the ephemeral port of the client
/// is usually unknown and must be guessed.
pub struct PortAllocatePools {
    pub buckets: Vec<u64>,
    allocated: usize,
    bit_len: u32,
    peak: usize,
}

impl Default for PortAllocatePools {
    fn default() -> Self {
        Self {
            buckets: vec![0; Self::bucket_size()],
            peak: Self::bucket_size() - 1,
            bit_len: Self::bit_len(),
            allocated: 0,
        }
    }
}

impl PortAllocatePools {
    /// compute bucket size.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::sessions::*;
    ///
    /// assert_eq!(PortAllocatePools::bucket_size(), 256);
    /// ```
    pub fn bucket_size() -> usize {
        (Self::capacity() as f32 / 64.0).ceil() as usize
    }

    /// compute bucket last bit max offset.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::sessions::*;
    ///
    /// assert_eq!(PortAllocatePools::bit_len(), 63);
    /// ```
    pub fn bit_len() -> u32 {
        (Self::capacity() as f32 % 64.0).ceil() as u32
    }

    /// get pools capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::sessions::Bit;
    /// use turn::sessions::PortAllocatePools;
    ///
    /// assert_eq!(PortAllocatePools::capacity(), 65535 - 49152);
    /// ```
    pub const fn capacity() -> usize {
        65535 - 49152
    }

    /// get port range.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::sessions::*;
    ///
    /// assert_eq!(PortAllocatePools::port_range(), 49152..65535);
    /// ```
    pub const fn port_range() -> Range<u16> {
        49152..65535
    }

    /// get pools allocated size.
    ///
    /// ```
    /// use turn::sessions::PortAllocatePools;
    ///
    /// let mut pools = PortAllocatePools::default();
    /// assert_eq!(pools.len(), 0);
    ///
    /// pools.alloc(None).unwrap();
    /// assert_eq!(pools.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.allocated
    }

    /// get pools allocated size is empty.
    ///
    /// ```
    /// use turn::sessions::PortAllocatePools;
    ///
    /// let mut pools = PortAllocatePools::default();
    /// assert_eq!(pools.len(), 0);
    /// assert_eq!(pools.is_empty(), true);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.allocated == 0
    }

    /// random assign a port.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::sessions::PortAllocatePools;
    ///
    /// let mut pool = PortAllocatePools::default();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// assert!(pool.alloc(None).is_some());
    /// ```
    pub fn alloc(&mut self, start_index: Option<usize>) -> Option<u16> {
        let mut index = None;
        let mut start =
            start_index.unwrap_or_else(|| thread_rng().gen_range(0..self.peak as u16) as usize);

        // When the partition lookup has gone through the entire partition list, the
        // lookup should be stopped, and the location where it should be stopped is
        // recorded here.
        let previous = if start == 0 { self.peak } else { start - 1 };

        loop {
            // Finds the first high position in the partition.
            if let Some(i) = {
                let bucket = self.buckets[start];
                let offset = if bucket < u64::MAX {
                    bucket.leading_ones()
                } else {
                    return None;
                };

                // Check to see if the jump is beyond the partition list or the lookup exceeds
                // the maximum length of the allocation table.
                if start == self.peak && offset > self.bit_len {
                    return None;
                }

                Some(offset)
            } {
                index = Some(i as usize);
                break;
            }

            // As long as it doesn't find it, it continues to re-find it from the next
            // partition.
            if start == self.peak {
                start = 0;
            } else {
                start += 1;
            }

            // Already gone through all partitions, lookup failed.
            if start == previous {
                break;
            }
        }

        // Writes to the partition, marking the current location as already allocated.
        let index = index?;
        self.set_bit(start, index, Bit::High);
        self.allocated += 1;

        // The actual port number is calculated from the partition offset position.
        let num = (start * 64 + index) as u16;
        let port = Self::port_range().start + num;
        Some(port)
    }

    /// write bit flag in the bucket.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::sessions::Bit;
    /// use turn::sessions::PortAllocatePools;
    ///
    /// let mut pool = PortAllocatePools::default();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// pool.set_bit(0, 0, Bit::High);
    /// pool.set_bit(0, 1, Bit::High);
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49154));
    /// assert_eq!(pool.alloc(Some(0)), Some(49155));
    /// ```
    pub fn set_bit(&mut self, bucket: usize, index: usize, bit: Bit) {
        let high_mask = 1 << (63 - index);
        let mask = match bit {
            Bit::Low => u64::MAX ^ high_mask,
            Bit::High => high_mask,
        };

        let value = self.buckets[bucket];
        self.buckets[bucket] = match bit {
            Bit::High => value | mask,
            Bit::Low => value & mask,
        };
    }

    /// restore port in the buckets.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::sessions::PortAllocatePools;
    ///
    /// let mut pool = PortAllocatePools::default();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// pool.restore(49152);
    /// pool.restore(49153);
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    /// ```
    pub fn restore(&mut self, port: u16) {
        assert!(Self::port_range().contains(&port));

        // Calculate the location in the partition from the port number.
        let offset = (port - Self::port_range().start) as usize;
        let bucket = offset / 64;
        let index = offset - (bucket * 64);

        // Gets the bit value in the port position in the partition, if it is low, no
        // processing is required.
        if {
            match (self.buckets[bucket] & (1 << (63 - index))) >> (63 - index) {
                0 => Bit::Low,
                1 => Bit::High,
                _ => panic!(),
            }
        } == Bit::Low
        {
            return;
        }

        self.set_bit(bucket, index, Bit::Low);
        self.allocated -= 1;
    }
}

/// Authentication information for the session.
///
/// Digest data is data that summarises usernames and passwords by means of
/// long-term authentication.
#[derive(Debug, Clone)]
pub struct Auth {
    pub username: String,
    pub password: String,
    pub digest: [u8; 16],
}

/// Assignment information for the session.
///
/// Sessions are all bound to only one port and one channel.
#[derive(Debug, Clone, Copy)]
pub struct Allocate {
    pub port: Option<u16>,
    pub channel: Option<u16>,
}

/// turn session information.
///
/// A user can have many sessions.
///
/// The default survival time for a session is 600 seconds.
#[derive(Debug, Clone)]
pub struct Session {
    pub auth: Auth,
    pub allocate: Allocate,
    pub expires: u64,
}

/// Each socket is assigned a nonce.
///
/// A nonce is a random string with a typical survival time of 3600 seconds.
#[derive(Debug, Clone)]
pub struct Nonce {
    pub nonce: String,
    pub expires: u64,
}

/// The identifier of the session or socket.
///
/// Each session needs to be identified by a combination of three pieces of
/// information: the socket address, the source interface, and the transport
/// protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub address: SocketAddr,
    pub interface: SocketAddr,
    pub transport: Transport,
}

/// A specially optimised timer.
///
/// This timer does not stack automatically and needs to be stacked externally
/// and manually.
/// 
/// ```
/// use turn::sessions::Timer;
/// 
/// let timer = Timer::default();
/// 
/// assert_eq!(timer.get(), 0);
/// assert_eq!(timer.add(), 1);
/// assert_eq!(timer.get(), 1);
/// ```
#[derive(Default)]
pub struct Timer(AtomicU64);

impl Timer {
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    pub fn add(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed) + 1
    }
}

pub struct Sessions<T> {
    sessions: RwLock<HashMap<Symbol, Session>>,
    // port allocate pool
    pap: Mutex<PortAllocatePools>,
    // port bind table
    pbt: RwLock<HashMap<u16, Symbol>>,
    // channel bind table
    cbt: RwLock<HashMap<(Symbol, u16), Symbol>>,
    // address nonce table
    ant: RwLock<HashMap<Symbol, Nonce>>,
    observer: T,
    timer: Timer,
}

impl<T: Observer + 'static> Sessions<T> {
    pub fn new(observer: T) -> Arc<Self> {
        let this = Arc::new(Self {
            sessions: RwLock::new(HashMap::with_capacity(PortAllocatePools::capacity())),
            pbt: RwLock::new(HashMap::with_capacity(PortAllocatePools::capacity())),
            cbt: RwLock::new(HashMap::with_capacity(PortAllocatePools::capacity())),
            ant: RwLock::new(HashMap::with_capacity(1024)),
            pap: Default::default(),
            timer: Default::default(),
            observer,
        });

        // This is a background thread that silently handles expiring sessions and
        // cleans up session information when it expires.
        let this_ = Arc::downgrade(&this);
        thread::spawn(move || {
            let mut keys = Vec::with_capacity(255);

            while let Some(this) = this_.upgrade() {
                // The timer advances one second and gets the current time offset.
                let now = this.timer.add();

                // This is the part that deletes the session information.
                {
                    // Finds sessions that have expired.
                    {
                        this.sessions
                            .read()
                            .iter()
                            .filter(|(_, v)| v.expires <= now)
                            .for_each(|(k, _)| keys.push(*k));
                    }

                    // Delete the expired sessions.
                    if !keys.is_empty() {
                        let mut sessions_lock = this.sessions.write();
                        let mut pbt_lock = this.pbt.write();
                        let mut cbt_lock = this.cbt.write();
                        let mut pap_lock = this.pap.lock();

                        keys.iter().for_each(|k| {
                            if let Some(session) = sessions_lock.remove(k) {
                                // Removes the session-bound port from the port binding table and
                                // releases the port back into the allocation pool.
                                if let Some(port) = session.allocate.port {
                                    pbt_lock.remove(&port);
                                    pap_lock.restore(port);
                                }

                                // Removes a session's binding relationship from the channel binding
                                // table.
                                if let Some(channel) = session.allocate.channel {
                                    cbt_lock.remove(&(*k, channel));
                                }

                                // Notifies that the external session has been closed.
                                this.observer.closed(k, &session.auth.username);
                            }
                        });

                        keys.clear();
                    }
                }

                // Because nonce does not follow session creation, nonce is created for each
                // socket, so nonce deletion is handled independently.
                {
                    this.ant
                        .read()
                        .iter()
                        .filter(|(_, v)| v.expires <= now)
                        .for_each(|(k, _)| keys.push(*k));

                    if !keys.is_empty() {
                        let mut ant_lock = this.ant.write();

                        keys.iter().for_each(|k| {
                            ant_lock.remove(k);
                        });

                        keys.clear();
                    }
                }

                // Fixing a second tick.
                sleep(Duration::from_secs(1));
            }
        });

        this
    }

    /// Get session for symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_trait::async_trait;
    /// use stun::Transport;
    /// use turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// #[async_trait]
    /// impl Observer for ObserverTest {
    ///     async fn get_password(
    ///         &self,
    ///         key: &Symbol,
    ///         username: &str,
    ///     ) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let symbol = Symbol {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::UDP,
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// assert!(sessions.get_session(&symbol).get_ref().is_none());
    ///
    /// pollster::block_on(sessions.get_digest(&symbol, "test", "test"));
    ///
    /// let lock = sessions.get_session(&symbol);
    /// let session = lock.get_ref().unwrap();
    /// assert_eq!(session.auth.username, "test");
    /// assert_eq!(session.auth.password, "test");
    /// assert_eq!(session.allocate.port, None);
    /// assert_eq!(session.allocate.channel, None);
    /// ```
    pub fn get_session<'a, 'b>(
        &'a self,
        key: &'b Symbol,
    ) -> ReadLock<'b, 'a, Symbol, HashMap<Symbol, Session>> {
        ReadLock {
            lock: self.sessions.read(),
            key,
        }
    }

    /// Get digest for symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_trait::async_trait;
    /// use stun::Transport;
    /// use turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// #[async_trait]
    /// impl Observer for ObserverTest {}
    ///
    /// let symbol = Symbol {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::UDP,
    /// };
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// let a = sessions.get_nonce(&symbol).get_ref().unwrap().clone();
    /// assert!(a.nonce.len() == 16);
    /// assert!(a.expires == 3600);
    /// 
    /// let b = sessions.get_nonce(&symbol).get_ref().unwrap().clone();
    /// assert_eq!(a.nonce, b.nonce);
    /// assert!(b.expires == 3600);
    /// ```
    pub fn get_nonce<'a, 'b>(
        &'a self,
        key: &'b Symbol,
    ) -> ReadLock<'b, 'a, Symbol, HashMap<Symbol, Nonce>> {
        // If no nonce is created, create a new one.
        {
            if !self.ant.read().contains_key(key) {
                self.ant.write().insert(
                    *key,
                    Nonce {
                        // Current time stacks for 3600 seconds.
                        expires: self.timer.get() + 3600,
                        // A random string of length 16.
                        nonce: {
                            let mut rng = thread_rng();
                            std::iter::repeat(())
                                .map(|_| rng.sample(Alphanumeric) as char)
                                .take(16)
                                .collect::<String>()
                                .to_lowercase()
                        },
                    },
                );
            }
        }

        ReadLock {
            lock: self.ant.read(),
            key,
        }
    }

    /// Get digest for symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_trait::async_trait;
    /// use stun::Transport;
    /// use turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// #[async_trait]
    /// impl Observer for ObserverTest {
    ///     async fn get_password(
    ///         &self,
    ///         key: &Symbol,
    ///         username: &str,
    ///     ) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let symbol = Symbol {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::UDP,
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// assert_eq!(
    ///     pollster::block_on(sessions.get_digest(&symbol, "test1", "test")),
    ///     None
    /// );
    ///
    /// assert_eq!(
    ///     pollster::block_on(sessions.get_digest(&symbol, "test", "test")),
    ///     Some(digest)
    /// );
    ///
    /// assert_eq!(
    ///     pollster::block_on(sessions.get_digest(&symbol, "test", "test")),
    ///     Some(digest)
    /// );
    /// ```
    pub async fn get_digest(&self, key: &Symbol, username: &str, realm: &str) -> Option<[u8; 16]> {
        // Already authenticated, get the cached digest directly.
        {
            if let Some(it) = self.sessions.read().get(key) {
                return Some(it.auth.digest);
            }
        }

        // Get the current user's password from an external observer and create a
        // digest.
        let password = self.observer.get_password(key, username).await?;
        let digest = long_key(&username, &password, realm);

        // Record a new session.
        {
            self.sessions.write().insert(
                *key,
                Session {
                    expires: self.timer.get() + 600,
                    auth: Auth {
                        username: username.to_string(),
                        password,
                        digest,
                    },
                    allocate: Allocate {
                        port: None,
                        channel: None,
                    },
                },
            );
        }

        Some(digest)
    }

    pub fn allocated(&self) -> usize {
        self.pap.lock().len()
    }

    /// Assign a port number to the session.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_trait::async_trait;
    /// use stun::Transport;
    /// use turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// #[async_trait]
    /// impl Observer for ObserverTest {
    ///     async fn get_password(
    ///         &self,
    ///         key: &Symbol,
    ///         username: &str,
    ///     ) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let symbol = Symbol {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::UDP,
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// pollster::block_on(sessions.get_digest(&symbol, "test", "test"));
    ///
    /// {
    ///     let lock = sessions.get_session(&symbol);
    ///     let session = lock.get_ref().unwrap();
    ///     assert_eq!(session.auth.username, "test");
    ///     assert_eq!(session.auth.password, "test");
    ///     assert_eq!(session.allocate.port, None);
    ///     assert_eq!(session.allocate.channel, None);
    /// }
    ///
    /// let port = sessions.allocate(&symbol).unwrap();
    /// {
    ///     let lock = sessions.get_session(&symbol);
    ///     let session = lock.get_ref().unwrap();
    ///     assert_eq!(session.auth.username, "test");
    ///     assert_eq!(session.auth.password, "test");
    ///     assert_eq!(session.allocate.port, Some(port));
    ///     assert_eq!(session.allocate.channel, None);
    /// }
    ///
    /// assert!(sessions.allocate(&symbol).is_none());
    /// ```
    pub fn allocate(&self, key: &Symbol) -> Option<u16> {
        let mut lock = self.sessions.write();
        let session = lock.get_mut(key)?;

        // If the port has already been allocated, re-allocation is not allowed.
        if session.allocate.port.is_some() {
            return None;
        }

        // Records the port assigned to the current session and resets the alive time.
        let port = self.pap.lock().alloc(None)?;
        session.expires = self.timer.get() + 600;
        session.allocate.port = Some(port);

        // Write the allocation port binding table.
        self.pbt.write().insert(port, *key);
        Some(port)
    }

    /// Binding a channel to the session.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_trait::async_trait;
    /// use stun::Transport;
    /// use turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// #[async_trait]
    /// impl Observer for ObserverTest {
    ///     async fn get_password(
    ///         &self,
    ///         key: &Symbol,
    ///         username: &str,
    ///     ) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let symbol = Symbol {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::UDP,
    /// };
    ///
    /// let peer_symbol = Symbol {
    ///     address: "127.0.0.1:8081".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::UDP,
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// pollster::block_on(sessions.get_digest(&symbol, "test", "test"));
    /// pollster::block_on(sessions.get_digest(&peer_symbol, "test", "test"));
    ///
    /// let port = sessions.allocate(&symbol).unwrap();
    /// let peer_port = sessions.allocate(&peer_symbol).unwrap();
    /// assert_eq!(
    ///     sessions
    ///         .get_session(&symbol)
    ///         .get_ref()
    ///         .unwrap()
    ///         .allocate
    ///         .channel,
    ///     None
    /// );
    ///
    /// assert_eq!(
    ///     sessions
    ///         .get_session(&peer_symbol)
    ///         .get_ref()
    ///         .unwrap()
    ///         .allocate
    ///         .channel,
    ///     None
    /// );
    ///
    /// assert!(sessions.bind_channel(&symbol, peer_port, 0x4000));
    /// assert!(sessions.bind_channel(&peer_symbol, port, 0x4000));
    /// assert_eq!(
    ///     sessions
    ///         .get_session(&symbol)
    ///         .get_ref()
    ///         .unwrap()
    ///         .allocate
    ///         .channel,
    ///     Some(0x4000)
    /// );
    ///
    /// assert_eq!(
    ///     sessions
    ///         .get_session(&peer_symbol)
    ///         .get_ref()
    ///         .unwrap()
    ///         .allocate
    ///         .channel,
    ///     Some(0x4000)
    /// );
    /// ```
    pub fn bind_channel(&self, key: &Symbol, port: u16, channel: u16) -> bool {
        // Finds the address of the bound opposing port.
        let peer = if let Some(it) = self.pbt.read().get(&port) {
            *it
        } else {
            return false;
        };

        // Checks if the current session is already bound to a channel, if it is already
        // bound and is not a duplicate binding, rejects the binding, if it is a normal
        // binding records the bound channel to the session.
        {
            let mut lock = self.sessions.write();
            if let Some(session) = lock.get_mut(key) {
                if let Some(it) = session.allocate.channel {
                    if it != channel {
                        return false;
                    }
                }

                session.allocate.channel = Some(channel);
            }
        }

        // Records the binding relationship to the channel binding mapping table.
        {
            self.cbt.write().insert((*key, channel), peer);
        }

        true
    }

    /// Gets the peer of the current session bound channel.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_trait::async_trait;
    /// use stun::Transport;
    /// use turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// #[async_trait]
    /// impl Observer for ObserverTest {
    ///     async fn get_password(
    ///         &self,
    ///         key: &Symbol,
    ///         username: &str,
    ///     ) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let symbol = Symbol {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::UDP,
    /// };
    ///
    /// let peer_symbol = Symbol {
    ///     address: "127.0.0.1:8081".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::UDP,
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// pollster::block_on(sessions.get_digest(&symbol, "test", "test"));
    /// pollster::block_on(sessions.get_digest(&peer_symbol, "test", "test"));
    ///
    /// let port = sessions.allocate(&symbol).unwrap();
    /// let peer_port = sessions.allocate(&peer_symbol).unwrap();
    ///
    /// assert!(sessions.bind_channel(&symbol, peer_port, 0x4000));
    /// assert!(sessions.bind_channel(&peer_symbol, port, 0x4000));
    /// assert_eq!(
    ///     sessions.get_channel_bind(&symbol, 0x4000),
    ///     Some(peer_symbol)
    /// );
    ///
    /// assert_eq!(
    ///     sessions.get_channel_bind(&peer_symbol, 0x4000),
    ///     Some(symbol)
    /// );
    /// ```
    pub fn get_channel_bind(&self, key: &Symbol, channel: u16) -> Option<Symbol> {
        self.cbt.read().get(&(*key, channel)).copied()
    }

    /// Get the address of the port binding.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_trait::async_trait;
    /// use stun::Transport;
    /// use turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// #[async_trait]
    /// impl Observer for ObserverTest {
    ///     async fn get_password(
    ///         &self,
    ///         key: &Symbol,
    ///         username: &str,
    ///     ) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let symbol = Symbol {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::UDP,
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// pollster::block_on(sessions.get_digest(&symbol, "test", "test"));
    ///
    /// let port = sessions.allocate(&symbol).unwrap();
    /// assert_eq!(sessions.get_port_bind(port), Some(symbol));
    /// ```
    pub fn get_port_bind(&self, port: u16) -> Option<Symbol> {
        self.pbt.read().get(&port).copied()
    }

    /// Refresh the session for symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_trait::async_trait;
    /// use stun::Transport;
    /// use turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// #[async_trait]
    /// impl Observer for ObserverTest {
    ///     async fn get_password(
    ///         &self,
    ///         key: &Symbol,
    ///         username: &str,
    ///     ) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let symbol = Symbol {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::UDP,
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// assert!(sessions.get_session(&symbol).get_ref().is_none());
    ///
    /// pollster::block_on(sessions.get_digest(&symbol, "test", "test"));
    ///
    /// let expires = sessions.get_session(&symbol).get_ref().unwrap().expires;
    /// assert!(expires == 600 || expires == 601 || expires == 602);
    ///
    /// assert!(sessions.refresh(&symbol, 0));
    /// std::thread::sleep(std::time::Duration::from_secs(2));
    ///
    /// assert!(sessions.get_session(&symbol).get_ref().is_none());
    /// ```
    pub fn refresh(&self, key: &Symbol, lifetime: u32) -> bool {
        if let Some(session) = self.sessions.write().get_mut(key) {
            session.expires = self.timer.get() + lifetime as u64;

            true
        } else {
            false
        }
    }
}
