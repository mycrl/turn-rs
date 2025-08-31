use std::ops::Range;

use rand::Rng;

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
///
/// # Test
///
/// ```
/// use std::collections::HashSet;
/// use turn_server_service::session::ports::*;
///
/// let mut pool = PortAllocator::default();
/// let mut ports = HashSet::with_capacity(PortAllocator::capacity());
///
/// while let Some(port) = pool.alloc(None) {
///     ports.insert(port);
/// }
///
/// assert_eq!(PortAllocator::capacity() + 1, ports.len());
/// ```
pub struct PortAllocator {
    pub buckets: Vec<u64>,
    allocated: usize,
    bit_len: u32,
    peak: usize,
}

impl Default for PortAllocator {
    fn default() -> Self {
        Self {
            buckets: vec![0; Self::bucket_size()],
            peak: Self::bucket_size() - 1,
            bit_len: Self::bit_len(),
            allocated: 0,
        }
    }
}

impl PortAllocator {
    /// compute bucket size.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::ports::*;
    ///
    /// assert_eq!(PortAllocator::bucket_size(), 256);
    /// ```
    pub fn bucket_size() -> usize {
        (Self::capacity() as f32 / 64.0).ceil() as usize
    }

    /// compute bucket last bit max offset.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::ports::*;
    ///
    /// assert_eq!(PortAllocator::bit_len(), 63);
    /// ```
    pub fn bit_len() -> u32 {
        (Self::capacity() as f32 % 64.0).ceil() as u32
    }

    /// get pools capacity.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::ports::*;
    ///
    /// assert_eq!(PortAllocator::capacity(), 65535 - 49152);
    /// ```
    pub const fn capacity() -> usize {
        65535 - 49152
    }

    /// get port range.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::ports::*;
    ///
    /// assert_eq!(PortAllocator::port_range(), 49152..65535);
    /// ```
    pub const fn port_range() -> Range<u16> {
        49152..65535
    }

    /// get pools allocated size.
    ///
    /// ```
    /// use turn_server_service::session::ports::*;
    ///
    /// let mut pools = PortAllocator::default();
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
    /// use turn_server_service::session::ports::*;
    ///
    /// let mut pools = PortAllocator::default();
    /// assert_eq!(pools.len(), 0);
    /// assert_eq!(pools.is_empty(), true);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.allocated == 0
    }

    /// random assign a port.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::ports::*;
    ///
    /// let mut pool = PortAllocator::default();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// assert!(pool.alloc(None).is_some());
    /// ```
    pub fn alloc(&mut self, start_index: Option<usize>) -> Option<u16> {
        let mut index = None;
        let mut start =
            start_index.unwrap_or_else(|| rand::rng().random_range(0..self.peak as u16) as usize);

        // When the partition lookup has gone through the entire partition list, the
        // lookup should be stopped, and the location where it should be stopped is
        // recorded here.
        let previous = if start == 0 { self.peak } else { start - 1 };

        loop {
            // Finds the first high position in the partition.
            if let Some(i) = {
                let bucket = self.buckets[start];
                if bucket < u64::MAX {
                    let offset = bucket.leading_ones();

                    // Check to see if the jump is beyond the partition list or the lookup exceeds
                    // the maximum length of the allocation table.
                    if start == self.peak && offset > self.bit_len {
                        None
                    } else {
                        Some(offset)
                    }
                } else {
                    None
                }
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
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::ports::*;
    ///
    /// let mut pool = PortAllocator::default();
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
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::ports::*;
    ///
    /// let mut pool = PortAllocator::default();
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
