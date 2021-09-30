pub mod receiver_report;
pub mod sender_report;
pub mod header;

use report::receiver::ReceiverReport;
use report::sender::SenderReport;
use header::Header;

use anyhow::Result;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use bytes::{
    BytesMut,
    Bytes,
    Buf 
};

/// RTCP packet type.
#[repr(u8)]
#[derive(TryFromPrimitive)]
pub enum PacketKind {
    SR = 0xC8,
    RR = 0xC9,
}

/// RTCP packet
pub enum Packet {
    SR(SenderReport),
    RR(ReceiverReport),
    // SDES(Sdes),
    // BYE(Bye),
    // APP(App),
}

pub struct Rtcp {
    /// rtcp fixed header.
    pub header: Header,
    /// rtcp other packet info.
    pub packet: Packet,
    /// padding data buf.
    pub padding: Option<Bytes>
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
    /// use std::convert::TryFrom;
    /// use rtcp::protocol::RtcpDecoder;
    ///
    /// let buffer = [
    ///     0x80, 0xc8, 0x00, 0x06, 0x79, 0x26, 0x69, 0x55,
    ///     0xe8, 0xe2, 0xe2, 0x17, 0xd4, 0x2f, 0x05, 0x91,
    ///     0x36, 0x01, 0xb0, 0xaf, 0x34, 0x85, 0x78, 0x5e,
    ///     0x2d, 0xbc, 0x2a, 0x98
    /// ];
    ///
    /// let mut decoder = RtcpDecoder::new();
    /// decoder.extend(&buffer);
    ///
    /// // decoder.accept().unwrap();
    /// ```
    pub fn extend(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }
    
    /// # Unit Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use rtcp::protocol::RtcpDecoder;
    ///
    /// let buffer = [
    ///     0x80, 0xc8, 0x00, 0x06, 0x79, 0x26, 0x69, 0x55,
    ///     0xe8, 0xe2, 0xe2, 0x17, 0xd4, 0x2f, 0x05, 0x91,
    ///     0x36, 0x01, 0xb0, 0xaf, 0x34, 0x85, 0x78, 0x5e,
    ///     0x2d, 0xbc, 0x2a, 0x98
    /// ];
    ///
    /// let mut decoder = RtcpDecoder::new();
    /// decoder.extend(&buffer);
    ///
    /// let rtcp = decoder.accept().unwrap();
    /// ```
    pub fn accept(&mut self) -> Result<Option<Rtcp>> {
        if self.buf.len() <= 8 {
            return Ok(None)
        }
        
        let size = Header::peek_len(&self.buf[..]);
        if self.buf.len() < size {
            return Ok(None)
        }
        
        let mut buf = self.buf.split_to(size).freeze();
        let header = Header::try_from(&buf[..])?;

        buf.advance(8);
        let pd_size = if header.padding {
            buf[buf.len() - 1] as usize
        } else {
            0
        };

        let pl_size = buf.len() - pd_size;
        let body = buf.split_to(pl_size);
        
        let padding = if pd_size > 0 {
            Some(buf)
        } else {
            None 
        };
        
        let packet = match header.pt {
            PacketKind::SR => Packet::SR(SenderReport::try_from(&body[..])?),
        };

        Ok(Some(Rtcp {
            header,
            packet,
            padding,
        }))
    }
}
