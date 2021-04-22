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
    buckets: [u64; 256]
}

impl RandomPort {
    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use rtp::extension::Extension;
    ///
    /// let buffer = [
    ///     0xbe, 0xde, 0x00, 0x02, 0x22, 0x5b, 0xb3, 0x33,
    ///     0x41, 0x00, 0x8b, 0x00
    /// ];
    /// 
    /// let mut writer = BytesMut::new();
    /// let extension = Extension {
    ///     data: vec![576434995, 1090554624],
    ///     kind: 48862,
    /// };
    /// 
    /// extension.into(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    pub fn new() -> Self {
        Self { buckets: [u64::MAX; 256] }
    }
    
    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use rtp::extension::Extension;
    ///
    /// let buffer = [
    ///     0xbe, 0xde, 0x00, 0x02, 0x22, 0x5b, 0xb3, 0x33,
    ///     0x41, 0x00, 0x8b, 0x00
    /// ];
    /// 
    /// let mut writer = BytesMut::new();
    /// let extension = Extension {
    ///     data: vec![576434995, 1090554624],
    ///     kind: 48862,
    /// };
    /// 
    /// extension.into(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    pub fn alloc(&mut self, si: Option<usize>) -> Option<u16> {
        let mut start = si.unwrap_or_else(|| Self::random() as usize);
        let mut index = None;

        let previous = if start == 0 {
            255
        } else {
            start - 1
        };

    loop {
        if let Some(i) = self.find_high(start) {
            index = Some(i as usize);
            break;
        }

        if start == 255 {
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
        Some((49152 + (start * 64 + bi)) as u16)
    }
    
    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use rtp::extension::Extension;
    ///
    /// let buffer = [
    ///     0xbe, 0xde, 0x00, 0x02, 0x22, 0x5b, 0xb3, 0x33,
    ///     0x41, 0x00, 0x8b, 0x00
    /// ];
    /// 
    /// let mut writer = BytesMut::new();
    /// let extension = Extension {
    ///     data: vec![576434995, 1090554624],
    ///     kind: 48862,
    /// };
    /// 
    /// extension.into(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
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
        
        if i == 255 && offset > 0 {
            return None
        }
        
        Some(offset)
    }

    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use rtp::extension::Extension;
    ///
    /// let buffer = [
    ///     0xbe, 0xde, 0x00, 0x02, 0x22, 0x5b, 0xb3, 0x33,
    ///     0x41, 0x00, 0x8b, 0x00
    /// ];
    /// 
    /// let mut writer = BytesMut::new();
    /// let extension = Extension {
    ///     data: vec![576434995, 1090554624],
    ///     kind: 48862,
    /// };
    /// 
    /// extension.into(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    pub fn write(&mut self, offset: usize, i: usize, bit: Bit) {
        let value = self.buckets[offset];
        let high_mask = 1 << (63 - i);
        let mask = match bit {
            Bit:Low => u64::MAX ^ high_mask,
            Bit::High => high_mask,
        };
        
        self.buckets[offset] = match bit {
            Bit::High => value | mask,
            Bit:Low => value & mask,
        };
    }
    
    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use rtp::extension::Extension;
    ///
    /// let buffer = [
    ///     0xbe, 0xde, 0x00, 0x02, 0x22, 0x5b, 0xb3, 0x33,
    ///     0x41, 0x00, 0x8b, 0x00
    /// ];
    /// 
    /// let mut writer = BytesMut::new();
    /// let extension = Extension {
    ///     data: vec![576434995, 1090554624],
    ///     kind: 48862,
    /// };
    /// 
    /// extension.into(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    pub fn restore(&mut self, port: u16) {
        assert!((49152..=65535).contains(&port));
        let offset = port - 49152;
        let bsize = offset / 64;
        let index = offset - (bsize * 64);
        self.write(bsize, index, Bit::High)
    }

    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use rtp::extension::Extension;
    ///
    /// let buffer = [
    ///     0xbe, 0xde, 0x00, 0x02, 0x22, 0x5b, 0xb3, 0x33,
    ///     0x41, 0x00, 0x8b, 0x00
    /// ];
    /// 
    /// let mut writer = BytesMut::new();
    /// let extension = Extension {
    ///     data: vec![576434995, 1090554624],
    ///     kind: 48862,
    /// };
    /// 
    /// extension.into(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    pub fn random() -> u16 {
        let mut rng = thread_rng();
        rng.gen_range(0, 256)
    }
}
