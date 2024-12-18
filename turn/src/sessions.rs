use std::{
    future::Future,
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

use ahash::{HashMap, HashMapExt, HashSet};
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use stun::util::long_key;

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
    /// use turn::router::ports::*;
    ///
    /// assert_eq!(bucket_size(), 256);
    /// ```
    pub fn bucket_size() -> usize {
        (Self::capacity() as f32 / 64.0).ceil() as usize
    }

    /// compute bucket last bit max offset.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::ports::*;
    ///
    /// assert_eq!(bit_len(), 63);
    /// ```
    pub fn bit_len() -> u32 {
        (Self::capacity() as f32 % 64.0).ceil() as u32
    }

    /// get pools capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::ports::Bit;
    /// use turn::router::ports::PortPools;
    ///
    /// assert_eq!(PortPools::capacity(), 65535 - 49152);
    /// ```
    pub const fn capacity() -> usize {
        65535 - 49152
    }

    /// get port range.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::ports::*;
    ///
    /// assert_eq!(port_range(), 49152..65535);
    /// ```
    pub const fn port_range() -> Range<u16> {
        49152..65535
    }

    /// get pools allocated size.
    ///
    /// ```
    /// use turn::router::ports::PortPools;
    ///
    /// let mut pools = PortPools::new();
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
    /// use turn::router::ports::PortPools;
    ///
    /// let mut pools = PortPools::new();
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
    /// use turn::router::ports::PortPools;
    ///
    /// let mut pool = PortPools::new();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// assert!(pool.alloc(None).is_some());
    /// ```
    pub fn alloc(&mut self, si: Option<usize>) -> Option<u16> {
        let mut start = si.unwrap_or_else(|| thread_rng().gen_range(0..self.peak as u16) as usize);
        let previous = if start == 0 { self.peak } else { start - 1 };
        let mut index = None;

        loop {
            if let Some(i) = self.find_high(start) {
                index = Some(i as usize);
                break;
            }

            if start == self.peak {
                start = 0;
            } else {
                start += 1;
            }

            if start == previous {
                break;
            }
        }

        let bi = match index {
            None => return None,
            Some(i) => i,
        };

        self.write(start, bi, Bit::High);
        self.allocated += 1;

        let num = (start * 64 + bi) as u16;
        let port = Self::port_range().start + num;
        Some(port)
    }

    /// find the high bit in the bucket.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::ports::PortPools;
    ///
    /// let mut pool = PortPools::new();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// assert_eq!(pool.find_high(0), Some(2));
    /// assert_eq!(pool.find_high(0), Some(2));
    /// assert_eq!(pool.find_high(1), Some(0));
    /// ```
    pub fn find_high(&self, i: usize) -> Option<u32> {
        let bucket = self.buckets[i];
        let offset = if bucket < u64::MAX {
            bucket.leading_ones()
        } else {
            return None;
        };

        if i == self.peak && offset > self.bit_len {
            return None;
        }

        Some(offset)
    }

    /// write bit flag in the bucket.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::ports::Bit;
    /// use turn::router::ports::PortPools;
    ///
    /// let mut pool = PortPools::new();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// pool.write(0, 0, Bit::High);
    /// pool.write(0, 1, Bit::High);
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49154));
    /// assert_eq!(pool.alloc(Some(0)), Some(49155));
    /// ```
    pub fn write(&mut self, offset: usize, i: usize, bit: Bit) {
        let bucket = self.buckets[offset];
        let high_mask = 1 << (63 - i);
        let mask = match bit {
            Bit::Low => u64::MAX ^ high_mask,
            Bit::High => high_mask,
        };

        self.buckets[offset] = match bit {
            Bit::High => bucket | mask,
            Bit::Low => bucket & mask,
        };
    }

    /// read bucket bit value.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::ports::Bit;
    /// use turn::router::ports::PortPools;
    ///
    /// let mut pool = PortPools::new();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// assert_eq!(pool.find_high(0), Some(2));
    /// assert_eq!(pool.find_high(1), Some(0));
    ///
    /// pool.write(0, 0, Bit::High);
    /// pool.write(0, 1, Bit::High);
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49154));
    /// assert_eq!(pool.alloc(Some(0)), Some(49155));
    ///
    /// pool.restore(49152);
    /// pool.restore(49153);
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    /// ```
    pub fn read(&self, o: usize, i: usize) -> Bit {
        match (self.buckets[o] & (1 << (63 - i))) >> (63 - i) {
            0 => Bit::Low,
            1 => Bit::High,
            _ => panic!(),
        }
    }

    /// restore port in the buckets.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn::router::ports::PortPools;
    ///
    /// let mut pool = PortPools::new();
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

        let offset = (port - Self::port_range().start) as usize;
        let bucket = offset / 64;
        let bit = offset - (bucket * 64);

        if self.read(bucket, bit) == Bit::Low {
            return;
        }

        self.write(bucket, bit, Bit::Low);
        self.allocated -= 1;
    }
}

#[derive(Debug, Clone)]
pub struct Auth {
    pub username: String,
    pub password: String,
    pub digest: [u8; 16],
}

#[derive(Debug, Clone, Copy)]
pub struct Allocate {
    pub port: Option<u16>,
    pub channel: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub nonce: String,
    pub auth: Auth,
    pub allocate: Allocate,
    pub expires: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transport {
    TCP,
    UDP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub address: SocketAddr,
    pub interface: SocketAddr,
    pub transport: Transport,
}

#[derive(Default)]
struct Timer(AtomicU64);

impl Timer {
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    pub fn add(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

pub struct Sessions {
    sessions: RwLock<HashMap<Symbol, Session>>,
    // user address table
    uat: RwLock<HashMap<String, HashSet<Symbol>>>,
    // port allocate pool
    pap: Mutex<PortAllocatePools>,
    timer: Timer,
    realm: String,
}

impl Sessions {
    pub fn new(realm: String) -> Arc<Self> {
        let this = Arc::new(Self {
            sessions: RwLock::new(HashMap::with_capacity(PortAllocatePools::capacity())),
            uat: RwLock::new(HashMap::with_capacity(PortAllocatePools::capacity())),
            pap: Default::default(),
            timer: Default::default(),
            realm,
        });

        let this_ = Arc::downgrade(&this);
        thread::spawn(move || {
            let mut keys = Vec::with_capacity(255);

            while let Some(this) = this_.upgrade() {
                let now = this.timer.add();

                {
                    this.sessions
                        .read()
                        .iter()
                        .filter(|(_, v)| v.expires <= now)
                        .for_each(|(k, _)| keys.push(*k));
                }

                if !keys.is_empty() {
                    let mut pap_lock = this.pap.lock();
                    let mut sessions_lock = this.sessions.write();
                    let mut uat_lock = this.uat.write();

                    keys.iter().for_each(|k| {
                        if let Some(session) = sessions_lock.remove(k) {
                            {
                                let mut remove = false;

                                if let Some(addrs) = uat_lock.get_mut(&session.auth.username) {
                                    if addrs.len() == 1 {
                                        remove = true;
                                    } else {
                                        addrs.remove(k);
                                    }
                                }

                                if remove {
                                    uat_lock.remove(&session.auth.username);
                                }
                            }

                            if let Some(port) = session.allocate.port {
                                pap_lock.restore(port);
                            }
                        }
                    });

                    keys.clear();
                }

                sleep(Duration::from_secs(1));
            }
        });

        this
    }

    pub fn get_session<'a, 'b>(
        &'a self,
        key: &'b Symbol,
    ) -> ReadLock<'b, 'a, Symbol, HashMap<Symbol, Session>> {
        ReadLock {
            lock: self.sessions.read(),
            key,
        }
    }

    pub fn get_user_addrs<'a, 'b>(
        &'a self,
        key: &'b String,
    ) -> ReadLock<'b, 'a, String, HashMap<String, HashSet<Symbol>>> {
        ReadLock {
            lock: self.uat.read(),
            key,
        }
    }

    pub async fn get_digest<F>(&self, key: &Symbol, func: F) -> Option<[u8; 16]>
    where
        F: Future<Output = Option<(String, String)>>,
    {
        {
            if let Some(it) = self.sessions.read().get(key) {
                return Some(it.auth.digest);
            }
        }

        let (username, password) = func.await?;
        let digest = long_key(&username, &password, &self.realm);

        self.uat
            .write()
            .entry(username.clone())
            .or_default()
            .insert(*key);

        {
            self.sessions.write().insert(
                *key,
                Session {
                    expires: self.timer.get() + 60,
                    auth: Auth {
                        username,
                        password,
                        digest,
                    },
                    allocate: Allocate {
                        port: None,
                        channel: None,
                    },
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

        Some(digest)
    }

    pub fn allocate(&self, key: &Symbol) -> Option<u16> {
        let port = self.pap.lock().alloc(None)?;

        {
            let mut lock = self.sessions.write();
            let session = lock.get_mut(key)?;
            session.expires = self.timer.get() + 600;
            session.allocate.port = Some(port);
        }

        Some(port)
    }

    pub fn bind_channel(&self, key: &Symbol) {
        
    }

    pub fn refresh(&self, key: &Symbol, lifetime: u32) -> bool {
        if let Some(session) = self.sessions.write().get_mut(key) {
            session.expires = self.timer.get() + lifetime as u64;

            true
        } else {
            false
        }
    }
}
