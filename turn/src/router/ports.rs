use super::Addr;
use core::panic;
use rand::{
    thread_rng,
    Rng,
};

use parking_lot::{
    RwLock,
    Mutex,
};

use std::{
    collections::HashMap,
    ops::Range,
};

/// Bit Flag
#[derive(PartialEq)]
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
pub struct PortPools {
    buckets: Vec<u64>,
    allocated: usize,
    bit_len: u32,
    peak: usize,
}

impl PortPools {
    pub fn new() -> Self {
        Self {
            buckets: vec![0; bucket_size()],
            peak: bucket_size() - 1,
            bit_len: bit_len(),
            allocated: 0,
        }
    }

    /// get pools capacity.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use ports::PortPools;
    /// use ports::Bit;
    ///
    /// let pools = PortPools::new();
    /// assert_eq!(pool.capacity(), 65535 - 49152);
    /// ```
    pub fn capacity(&self) -> usize {
        capacity()
    }

    /// get pools allocated size.
    ///
    /// ```ignore
    /// use ports::PortPools;
    /// use ports::Bit;
    ///
    /// let pools = PortPools::new();
    /// assert_eq!(pool.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.allocated
    }

    /// random assign a port.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use ports::PortPools;
    ///
    /// let mut pool = PortPools::new();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// assert!(pool.alloc(None).is_some());
    /// ```
    #[rustfmt::skip]
    pub fn alloc(&mut self, si: Option<usize>) -> Option<u16> {
        let mut start = si.unwrap_or_else(|| self.random() as usize);
        let previous = if start == 0 { self.peak } else { start - 1 };
        let mut index = None;

    // warn: loop
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
        let port = port_range().start + num;
        Some(port)
    }

    /// find the high bit in the bucket.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use ports::PortPools;
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
    /// ```ignore
    /// use ports::PortPools;
    /// use ports::Bit;
    ///
    /// let mut pool = PortPools::new();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// pool.write(0, 0, Bit::High);
    /// pool.write(0, 1, Bit::High);
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
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
    /// ```ignore
    /// use ports::PortPools;
    /// use ports::Bit;
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
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
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
    /// ```ignore
    /// use ports::PortPools;
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
        assert!(port_range().contains(&port));
        let offset = (port - port_range().start) as usize;
        let bucket = offset / 64;
        let bit = offset - (bucket * 64);

        if self.read(bucket, bit) == Bit::Low {
            return;
        }

        self.write(bucket, bit, Bit::Low);
        self.allocated -= 1;
    }

    /// get random buckets index.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use ports::PortPools;
    ///
    /// let range = 49152..65535;
    /// let max = PortPools::bucket_size(&range) as u16;
    /// let pool = PortPools::new(range);
    ///
    /// let index = pool.random();
    /// assert!((0..max - 1).contains(&index));
    /// ```
    pub fn random(&self) -> u16 {
        let mut rng = thread_rng();
        rng.gen_range(0..self.peak as u16)
    }
}

/// port table.
pub struct Ports {
    pools: Mutex<PortPools>,
    map: RwLock<HashMap<u16, Addr>>,
    bounds: RwLock<HashMap<Addr, HashMap<Addr, u16>>>,
}

impl Ports {
    pub fn new() -> Self {
        Self {
            bounds: RwLock::new(HashMap::with_capacity(capacity())),
            map: RwLock::new(HashMap::with_capacity(capacity())),
            pools: Mutex::new(PortPools::new()),
        }
    }

    /// get ports capacity.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pools = Ports::new();
    /// assert_eq!(pools.capacity(), 65535 - 49152);
    /// ```
    pub fn capacity(&self) -> usize {
        self.pools.lock().capacity()
    }

    /// get ports allocated size.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pools = Ports::new();
    /// assert_eq!(pools.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.pools.lock().len()
    }

    /// get address from port.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pools = Ports::new();
    /// let port = pools.alloc().unwrap();
    ///
    /// assert!(pools.get(port).is_some());
    /// ```
    pub fn get(&self, p: u16) -> Option<Addr> {
        self.map.read().get(&p).cloned()
    }

    /// get address bound port.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pools = Ports::new();
    /// let port = pools.alloc().unwrap();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// assert!(pools.bound(&addr, port).is_some());
    /// assert!(pools.get_bound(&addr, &addr).is_none());
    /// ```
    pub fn get_bound(&self, a: &Addr, p: &Addr) -> Option<u16> {
        self.bounds.read().get(p)?.get(a).cloned()
    }

    /// allocate port in ports.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pools = Ports::new();
    /// assert_eq!(pools.alloc().is_some(), true);
    /// ```
    pub fn alloc(&self, a: &Addr) -> Option<u16> {
        let port = self.pools.lock().alloc(None)?;
        self.map.write().insert(port, a.clone());
        Some(port)
    }

    /// bound address and peer port.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pools = Ports::new();
    /// let port = pools.alloc().unwrap();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// assert!(pools.bound(&addr, port).is_some());
    /// ```
    pub fn bound(&self, a: &Addr, port: u16) -> Option<()> {
        let peer = self.map.read().get(&port)?.clone();
        self.bounds
            .write()
            .entry(a.clone())
            .or_insert_with(|| HashMap::with_capacity(10))
            .entry(peer)
            .or_insert(port);
        Some(())
    }

    /// bound address and peer port.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pools = Ports::new();
    /// let port = pools.alloc().unwrap();
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///
    /// assert!(pools.bound(&addr, port).is_some());
    /// assert!(pools.remove(&addr, &vec![port]).is_some());
    /// ```
    pub fn remove(&self, a: &Addr, ports: &[u16]) -> Option<()> {
        let mut pools = self.pools.lock();
        let mut map = self.map.write();

        for p in ports {
            pools.restore(*p);
            map.remove(p);
        }

        self.bounds.write().remove(a);
        Some(())
    }
}

/// compute bucket size.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(bucket_size(), 256);
/// ```
fn bucket_size() -> usize {
    (capacity() as f32 / 64.0).ceil() as usize
}

/// compute bucket last bit max offset.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(bit_len(), 63);
/// ```
fn bit_len() -> u32 {
    (capacity() as f32 % 64.0).ceil() as u32
}

/// compute capacity.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(capacity(), 65535 - 49152);
/// ```
pub const fn capacity() -> usize {
    65535 - 49152
}

/// get port range.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(port_range(), 65535..49152);
/// ```
pub const fn port_range() -> Range<u16> {
    49152..65535
}
