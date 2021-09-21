pub mod sr;
pub mod header;

use sr::Sr;
use header::Header;

use anyhow::Result;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use bytes::BytesMut;

/// RTCP packet type.
#[repr(u8)]
#[derive(TryFromPrimitive)]
pub enum PacketKind {
    SR = 0xC8
}

/// RTCP packet
pub enum Packet {
    SR(Sr),
    // RR(Rr),
    // SDES(Sdes),
    // BYE(Bye),
    // APP(App),
}

pub struct Rtcp {
    /// rtcp fixed header.
    pub header: Header,
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
    pub fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(4096)
        }
    }
    
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
    pub fn extend(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }
    
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
    pub fn accept(&mut self) -> Result<Option<Rtcp>> {
        if self.buf.len() <= 8 {
            return Ok(None)
        }
        
        let size = Header::peek_len(&self.buf[..]);
        if self.buf.len() < size {
            return Ok(None)
        }
        
        let package = &self.buf[..size];
        let payload = &self.buf[8..size];

        let header = Header::try_from(package)?;
        let packet = match header.pt {
            PacketKind::SR => Packet::SR(Sr::try_from(payload)?),
        };

        Ok(Some(Rtcp {
            header,
            packet
        }))
    }
}
