use std::convert::TryFrom;
use super::PacketKind;

const VERSION_MASK: u8 = 0b11000000;
const PADDING_MASK: u8 = 0b00100000;
const RC_MASK: u8 = 0b00011111;

pub struct Header {
    /// version (V): 2 bits
    /// Identifies the version of RTP, which is the same in RTCP packets
    /// as in RTP data packets.  The version defined by this specification
    /// is two (2).
    pub version: u8,
    /// padding (P): 1 bit
    /// If the padding bit is set, this individual RTCP packet contains
    /// some additional padding octets at the end which are not part of
    /// the control information but are included in the length field.  The
    /// last octet of the padding is a count of how many padding octets
    /// should be ignored, including itself (it will be a multiple of
    /// four).  Padding may be needed by some encryption algorithms with
    /// fixed block sizes.  In a compound RTCP packet, padding is only
    /// required on one individual packet because the compound packet is
    /// encrypted as a whole for the method in rfc3550 section-9.1.  
    /// Thus, padding MUST only be added to the last individual packet, 
    /// and if padding is added to that packet, the padding bit MUST be set 
    /// only on that packet.  This convention aids the header validity 
    /// checks described in rfc3550 A.2 and allows detection of packets 
    /// from some early implementations that incorrectly set the padding 
    /// bit on the first individual packet and add padding to the last 
    /// individual packet.
    pub padding: bool,
    /// reception report count (RC): 5 bits
    /// The number of reception report blocks contained in this packet.  A
    /// value of zero is valid.
    pub rc: u8,
    /// packet type (PT): 8 bits
    /// Contains the constant 200 to identify this as an RTCP SR packet. 
    pub pt: PacketKind,
    /// SSRC: 32 bits
    /// The synchronization source identifier for the originator of this
    /// SR packet.
    pub ssrc: u32,
}

impl Header {
    /// length: 16 bits
    /// The length of this RTCP packet in 32-bit words minus one,
    /// including the header and any padding.  (The offset of one makes
    /// zero a valid length and avoids a possible infinite loop in
    /// scanning a compound RTCP packet, while counting 32-bit words
    /// avoids a validity check for a multiple of 4.)
    ///
    /// # Unit Test
    ///
    /// ```
    /// use rtcp::protocol::header::Header;
    ///
    /// let buffer = [
    ///     0x80, 0xc8, 0x00, 0x06, 0x79, 0x26, 0x69, 0x55,
    ///     0xe8, 0xe2, 0xe2, 0x17, 0xd4, 0x2f, 0x05, 0x91,
    ///     0x36, 0x01, 0xb0, 0xaf, 0x34, 0x85, 0x78, 0x5e,
    ///     0x2d, 0xbc, 0x2a, 0x98
    /// ];
    ///
    /// let len = Header::peek_len(&buffer);
    /// assert_eq!(len, 28);
    /// ```
    pub fn peek_len(buf: &[u8]) -> usize {
        assert!(buf.len() >= 4);
        let size = u16::from_be_bytes([
            buf[2], 
            buf[3]
        ]) as usize;
        (size + 1) * 4
    }
}

impl TryFrom<&[u8]> for Header {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use rtcp::protocol::header::Header;
    ///
    /// let buffer = [
    ///     0x80, 0xc8, 0x00, 0x06, 0x79, 0x26, 0x69, 0x55,
    ///     0xe8, 0xe2, 0xe2, 0x17, 0xd4, 0x2f, 0x05, 0x91,
    ///     0x36, 0x01, 0xb0, 0xaf, 0x34, 0x85, 0x78, 0x5e,
    ///     0x2d, 0xbc, 0x2a, 0x98
    /// ];
    ///
    /// let len = Header::peek_len(&buffer);
    /// assert_eq!(len, 28);
    /// ```
    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        assert!(buf.len() >= 8);

        let rc = buf[0] & RC_MASK;
        let version = (buf[0] & VERSION_MASK) >> 6;
        let padding = ((buf[0] & PADDING_MASK) >> 5) == 1;
        let pt = PacketKind::try_from(buf[1])?;
        let ssrc = u32::from_be_bytes([
            buf[4], 
            buf[5],
            buf[6], 
            buf[7],
        ]);
        
        Ok(Self {
            version,
            padding,
            ssrc,
            pt,
            rc
        })
    }
}
