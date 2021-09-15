mod sr;

use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use anyhow::Result;
use bytes::{
    BytesMut,
    Buf
};

const VERSION_MASK: u8 = 0b11000000;
const PADDING_MASK: u8 = 0b00100000;
const RC_MASK: u8 = 0b00011111;

/// RTCP packet type.
#[repr(u8)]
#[derive(TryFromPrimitive)]
pub enum PacketKind {
    SR = 0xC8
}

/// RTCP packet
pub enum Packet {
    SR(sr::Sr),
    // RR(Rr),
    // SDES(Sdes),
    // BYE(Bye),
    // APP(App),
}

pub struct Rtcp {
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
    /// length: 16 bits
    /// The length of this RTCP packet in 32-bit words minus one,
    /// including the header and any padding.  (The offset of one makes
    /// zero a valid length and avoids a possible infinite loop in
    /// scanning a compound RTCP packet, while counting 32-bit words
    /// avoids a validity check for a multiple of 4.)
    pub length: u16,
    /// SSRC: 32 bits
    /// The synchronization source identifier for the originator of this
    /// SR packet.
    pub ssrc: u32,
    /// rtcp other packet info.
    pub packet: Packet
}

/// RCTP decoder.
pub struct RtcpDecoder {
    buf: BytesMut
}

impl RtcpDecoder {
    /// Create rtcp decoder,
    /// allocate in 4KB inner buffer.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use rtp::header::Header;
    ///
    /// let buffer = [
    ///     0x90, 0x72, 0x04, 0xf1, 0xf8, 0x87, 0x3f, 0xad, 0x67, 0xfe,
    ///     0x9d, 0xfc
    /// ];
    /// 
    /// let mut writer = BytesMut::new();
    /// let header = Header {
    ///     version: 2,
    ///     padding: false,
    ///     extension: true,
    ///     marker: false,
    ///     payload_kind: 114,
    ///     sequence_number: 1265,
    ///     timestamp: 4169613229,
    ///     ssrc: 1744739836,
    ///     csrc_list: Vec::new(),
    /// };
    /// 
    /// 
    /// header.into_to_bytes(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    pub fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(4096)
        }
    }
    
    pub fn accept(&mut self, chunk: &[u8]) -> Result<Option<Rtcp>> {
        self.buf.extend_from_slice(chunk);
        
        // first, get packet size.
        let length = (u16::from_be_bytes([
            self.buf[2], 
            self.buf[3]
        ]) + 1) * 4;
        
        if self.buf.len() < length as usize {
            return Ok(None)
        }
        
        let rc = self.buf[0] & RC_MASK;
        let version = (self.buf[0] & VERSION_MASK) >> 6;
        let padding = ((self.buf[0] & PADDING_MASK) >> 5) == 1;
        let pt = PacketKind::try_from(self.buf[1])?;
        
        self.buf.advance(4);
        
        let ssrc = self.buf.get_u32();
        let packet = match pt {
            PacketKind::SR => Packet::SR(sr::Sr::try_from(&mut self.buf)?),
        };

        Ok(Some(Rtcp {
            version,
            padding,
            length,
            packet,
            ssrc,
            pt,
            rc
        }))
    }
}
