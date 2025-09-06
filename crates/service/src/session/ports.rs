use std::str::FromStr;

use rand::Rng;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRange {
    start: u16,
    end: u16,
}

impl PortRange {
    pub fn size(&self) -> usize {
        (self.end - self.start) as usize
    }

    pub fn contains(&self, port: u16) -> bool {
        port >= self.start && port <= self.end
    }
}

impl Default for PortRange {
    fn default() -> Self {
        Self {
            start: 49152,
            end: 65535,
        }
    }
}

impl From<std::ops::Range<u16>> for PortRange {
    fn from(range: std::ops::Range<u16>) -> Self {
        assert!(range.start <= range.end);

        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl ToString for PortRange {
    fn to_string(&self) -> String {
        format!("{}..{}", self.start, self.end)
    }
}

#[derive(Debug)]
pub struct PortRangeParseError(String);

impl std::error::Error for PortRangeParseError {}

impl std::fmt::Display for PortRangeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<std::num::ParseIntError> for PortRangeParseError {
    fn from(error: std::num::ParseIntError) -> Self {
        PortRangeParseError(error.to_string())
    }
}

impl FromStr for PortRange {
    type Err = PortRangeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (start, end) = s
            .split_once("..")
            .ok_or(PortRangeParseError(s.to_string()))?;

        Ok(Self {
            start: start.parse()?,
            end: end.parse()?,
        })
    }
}

#[cfg(feature = "serde")]
impl Serialize for PortRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for PortRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from_str(&s).map_err(|e| serde::de::Error::custom(e.0))?)
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
///
/// # Test
///
/// ```
/// use std::collections::HashSet;
/// use turn_service::session::ports::*;
///
/// let mut pool = PortAllocator::default();
/// let mut ports = HashSet::with_capacity(PortAllocator::default().capacity());
///
/// while let Some(port) = pool.alloc(None) {
///     ports.insert(port);
/// }
///
/// assert_eq!(PortAllocator::default().capacity() + 1, ports.len());
/// ```
pub struct PortAllocator {
    port_range: PortRange,
    buckets: Vec<u64>,
    allocated: usize,
    bit_len: u32,
    max_offset: usize,
}

impl Default for PortAllocator {
    fn default() -> Self {
        Self::new(PortRange::default())
    }
}

impl PortAllocator {
    pub fn new(port_range: PortRange) -> Self {
        let capacity = port_range.size();
        let bucket_size = (capacity as f32 / 64.0).ceil() as usize;

        Self {
            bit_len: (capacity as f32 % 64.0).ceil() as u32,
            buckets: vec![0; bucket_size],
            max_offset: bucket_size - 1,
            allocated: 0,
            port_range,
        }
    }

    /// get pools capacity.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_service::session::ports::*;
    ///
    /// assert_eq!(PortAllocator::default().capacity(), 65535 - 49152);
    /// ```
    pub fn capacity(&self) -> usize {
        self.port_range.size()
    }

    /// get port range.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_service::session::ports::*;
    ///
    /// let pool = PortAllocator::default();
    ///
    /// assert_eq!(pool.port_range().start, 49152);
    /// assert_eq!(pool.port_range().end, 65535);
    ///
    /// let pool = PortAllocator::new(50000..60000);
    ///
    /// assert_eq!(pool.port_range().start, 50000);
    /// assert_eq!(pool.port_range().end, 60000);
    /// ```
    pub fn port_range(&self) -> &PortRange {
        &self.port_range
    }

    /// get pools allocated size.
    ///
    /// ```
    /// use turn_service::session::ports::*;
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
    /// use turn_service::session::ports::*;
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
    /// use turn_service::session::ports::*;
    ///
    /// let mut pool = PortAllocator::default();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// assert!(pool.alloc(None).is_some());
    /// ```
    pub fn alloc(&mut self, start: Option<usize>) -> Option<u16> {
        let mut index = None;
        let mut offset =
            start.unwrap_or_else(|| rand::rng().random_range(0..self.max_offset) as usize);

        // When the partition lookup has gone through the entire partition list, the
        // lookup should be stopped, and the location where it should be stopped is
        // recorded here.
        let previous = if offset == 0 {
            self.max_offset
        } else {
            offset - 1
        };

        loop {
            // Finds the first high position in the partition.
            if let Some(i) = {
                let bucket = self.buckets[offset];
                if bucket < u64::MAX {
                    let idx = bucket.leading_ones();

                    // Check to see if the jump is beyond the partition list or the lookup exceeds
                    // the maximum length of the allocation table.
                    if offset == self.max_offset && idx > self.bit_len {
                        None
                    } else {
                        Some(idx)
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
            if offset == self.max_offset {
                offset = 0;
            } else {
                offset += 1;
            }

            // Already gone through all partitions, lookup failed.
            if offset == previous {
                break;
            }
        }

        // Writes to the partition, marking the current location as already allocated.
        let index = index?;
        self.set_bit(offset, index, Bit::High);
        self.allocated += 1;

        // The actual port number is calculated from the partition offset position.
        let num = (offset * 64 + index) as u16;
        let port = self.port_range.start + num;
        Some(port)
    }

    /// write bit flag in the bucket.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_service::session::ports::*;
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
    /// use turn_service::session::ports::*;
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
        assert!(self.port_range.contains(port));

        // Calculate the location in the partition from the port number.
        let offset = (port - self.port_range.start) as usize;
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
