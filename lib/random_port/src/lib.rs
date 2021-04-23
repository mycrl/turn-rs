use std::ops::Range;
use rand::{
    thread_rng,
    Rng
};

/// Bit Flag
pub enum Bit {
    Low,
    High
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
pub struct RandomPort {
    buckets: Vec<u64>,
    range: Range<u16>,
    high: usize,
}

impl RandomPort {
    pub fn new(range: Range<u16>) -> Self {
        let size = Self::bucket_size(&range);
        Self { 
            buckets: vec![u64::MAX; size],
            high: size - 1,
            range
        }
    }
    
    /// random assign a port.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use random_port::RandomPort;
    ///
    /// let range = 49152..65535;
    /// let mut pool = RandomPort::new(range);
    /// 
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    /// 
    /// assert!(pool.alloc(None).is_some());
    /// ```
    pub fn alloc(&mut self, si: Option<usize>) -> Option<u16> {
        let mut start = si.unwrap_or_else(|| self.random() as usize);
        let mut index = None;

        let previous = if start == 0 {
            self.high
        } else {
            start - 1
        };

    loop {
        if let Some(i) = self.find_high(start) {
            index = Some(i as usize);
            break;
        }

        if start == self.high {
            start = 0;
        } else {
            start += 1;
        }

        if start == previous {
            break;
        }
    }

        let bi = match index {
            Some(i) => i,
            None => return None
        };

        self.write(start, bi, Bit::Low);
        Some(self.range.start + (start * 64 + bi) as u16)
    }
    
    /// find buckets high bit.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use random_port::RandomPort;
    ///
    /// let range = 49152..65535;
    /// let mut pool = RandomPort::new(range);
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    /// 
    /// assert_eq!(pool.find_high(0), Some(2));
    /// assert_eq!(pool.find_high(0), Some(2));
    /// assert_eq!(pool.find_high(1), Some(0));
    /// ```
    pub fn find_high(&self, i: usize) -> Option<u32> {
        let value = self.buckets[i];
        let offset = if value != u64::MIN {
            value.leading_zeros()
        } else {
            return None
        };
        
        if offset == 8 {
            return None
        }
        
        if i == self.high && offset > 0 {
            return None
        }
        
        Some(offset)
    }

    /// write bit flag in bucket.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use random_port::RandomPort;
    /// use random_port::Bit;
    ///
    /// let range = 49152..65535;
    /// let mut pool = RandomPort::new(range);
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
        let value = self.buckets[offset];
        let high_mask = 1 << (63 - i);
        let mask = match bit {
            Bit::Low => u64::MAX ^ high_mask,
            Bit::High => high_mask,
        };
        
        self.buckets[offset] = match bit {
            Bit::High => value | mask,
            Bit::Low => value & mask,
        };
    }

    /// restore port in buckets.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use random_port::RandomPort;
    ///
    /// let range = 49152..65535;
    /// let mut pool = RandomPort::new(range);
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
        assert!(self.range.contains(&port));
        let offset = (port - self.range.start) as usize;
        let bsize = offset / 64;
        let index = offset - (bsize * 64);
        self.write(bsize, index, Bit::High)
    }

    /// 
    ///
    /// # Unit Test
    ///
    /// ```
    /// use random_port::RandomPort;
    ///
    /// let range = 49152..65535;
    /// let max = RandomPort::bucket_size(&range) as u16;
    /// let pool = RandomPort::new(range);
    /// 
    /// let index = pool.random();
    /// assert!((0..max - 1).contains(&index));
    /// ```
    pub fn random(&self) -> u16 {
        let mut rng = thread_rng();
        rng.gen_range(0, self.high as u16)
    }

    /// 
    ///
    /// # Unit Test
    ///
    /// ```
    /// use random_port::RandomPort;
    ///
    /// let range = 49152..65535;
    /// let size = RandomPort::bucket_size(&range);
    /// assert_eq!(size, 256);
    /// ```
    pub fn bucket_size(range: &Range<u16>) -> usize {
        ((range.end - range.start) as f32 / 64.0).ceil() as usize
    }
}
