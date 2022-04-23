pub mod sdes;
pub mod sr;
pub mod rr;

pub use rr::Rr;
pub use sr::Sr;

use anyhow::Result;
use std::convert::TryFrom;
use num_enum::TryFromPrimitive;
use bytes::{
    BytesMut,
    Buf 
};

pub const VERSION_MASK: u8 = 0b11000000;
pub const PADDING_MASK: u8 = 0b00100000;
pub const RC_MASK: u8 = 0b00011111;

/// RTCP packet type.
#[repr(u8)]
#[derive(PartialEq, Eq, Debug)]
#[derive(TryFromPrimitive)]
pub enum PacketKind {
    SR      = 0xC8,
    RR      = 0xC9,
    SDES    = 0xCA,
}

pub struct Source {
    /// SSRC_n (source identifier): 32 bits
    /// The SSRC identifier of the source to which the information in this
    /// reception report block pertains.
    pub identifier: u32,
    /// fraction lost: 8 bits
    /// The fraction of RTP data packets from source SSRC_n lost since the
    /// previous SR or RR packet was sent, expressed as a fixed point
    /// number with the binary point at the left edge of the field.  (That
    /// is equivalent to taking the integer part after multiplying the
    /// loss fraction by 256.)  This fraction is defined to be the number
    /// of packets lost divided by the number of packets expected, as
    /// defined in the next paragraph.  If the loss is negative due to 
    /// duplicates, the fraction lost is set to zero.  Note that a 
    /// receiver cannot tell whether any packets were lost after the last 
    /// one received, and that there will be no reception report block 
    /// issued for a source if all packets from that source sent during 
    /// the last reporting interval have been lost.
    pub fraction_lost: u8,
    /// cumulative number of packets lost: 24 bits
    /// The total number of RTP data packets from source SSRC_n that have
    /// been lost since the beginning of reception.  This number is
    /// defined to be the number of packets expected less the number of
    /// packets actually received, where the number of packets received
    /// includes any which are late or duplicates.  Thus, packets that
    /// arrive late are not counted as lost, and the loss may be negative
    /// if there are duplicates.  The number of packets expected is
    /// defined to be the extended last sequence number received, as
    /// defined next, less the initial sequence number received.
    pub cnopl: u32,
    /// extended highest sequence number received: 32 bits
    /// The low 16 bits contain the highest sequence number received in an
    /// RTP data packet from source SSRC_n, and the most significant 16
    /// bits extend that sequence number with the corresponding count of
    /// sequence number cycles, which may be maintained according to the
    /// algorithm in Appendix A.1.  Note that different receivers within
    /// the same session will generate different extensions to the
    /// sequence number if their start times differ significantly.
    pub ehsnr: u32,
    /// interarrival jitter: 32 bits
    /// An estimate of the statistical variance of the RTP data packet
    /// interarrival time, measured in timestamp units and expressed as an
    /// unsigned integer.  The interarrival jitter J is defined to be the
    /// mean deviation (smoothed absolute value) of the difference D in
    /// packet spacing at the receiver compared to the sender for a pair
    /// of packets.  As shown in the equation below, this is equivalent to
    /// the difference in the "relative transit time" for the two packets;
    /// the relative transit time is the difference between a packet's RTP
    /// timestamp and the receiver's clock at the time of arrival,
    /// measured in the same units.
    /// 
    /// If Si is the RTP timestamp from packet i, and Ri is the time of
    /// arrival in RTP timestamp units for packet i, then for two packets
    /// i and j, D may be expressed as
    /// 
    /// D(i,j) = (Rj - Ri) - (Sj - Si) = (Rj - Sj) - (Ri - Si)
    /// 
    /// The interarrival jitter SHOULD be calculated continuously as each
    /// data packet i is received from source SSRC_n, using this
    /// difference D for that packet and the previous packet i-1 in order
    /// of arrival (not necessarily in sequence), according to the formula
    /// 
    /// J(i) = J(i-1) + (|D(i-1,i)| - J(i-1))/16
    /// 
    /// Whenever a reception report is issued, the current value of J is
    /// sampled.
    /// 
    /// The jitter calculation MUST conform to the formula specified here
    /// in order to allow profile-independent monitors to make valid
    /// interpretations of reports coming from different implementations.
    /// This algorithm is the optimal first-order estimator and the gain
    /// parameter 1/16 gives a good noise reduction ratio while
    /// maintaining a reasonable rate of convergence 
    /// [22](https://datatracker.ietf.org/doc/html/rfc3550#ref-22). A sample
    /// implementation is shown in 
    /// [Appendix A.8](https://datatracker.ietf.org/doc/html/rfc3550#appendix-A.8).  
    /// See [Section 6.4.4](https://datatracker.ietf.org/doc/html/rfc3550#section-6.4.4) 
    /// for a discussion of the effects of varying packet duration and delay
    /// before transmission.
    pub interarrival_jitter: u32,
    /// last SR timestamp (LSR): 32 bits
    /// The middle 32 bits out of 64 in the NTP timestamp received as part 
    /// of the most recent RTCP sender report (SR) packet from source SSRC_n.  
    /// If no SR has been received yet, the field is set to zero.
    pub lsr: u32,
    /// delay since last SR (DLSR): 32 bits
    /// The delay, expressed in units of 1/65536 seconds, between
    /// receiving the last SR packet from source SSRC_n and sending this
    /// reception report block.  If no SR packet has been received yet
    /// from SSRC_n, the DLSR field is set to zero.
    /// 
    /// Let SSRC_r denote the receiver issuing this receiver report.
    /// Source SSRC_n can compute the round-trip propagation delay to
    /// SSRC_r by recording the time A when this reception report block is
    /// received.  It calculates the total round-trip time A-LSR using the
    /// last SR timestamp (LSR) field, and then subtracting this field to
    /// leave the round-trip propagation delay as (A - LSR - DLSR).  This
    /// is illustrated in Fig. 2.  Times are shown in both a hexadecimal
    /// representation of the 32-bit fields and the equivalent floating-
    /// point decimal representation.  Colons indicate a 32-bit field
    /// divided into a 16-bit integer part and 16-bit fraction part.
    /// 
    /// This may be used as an approximate measure of distance to cluster
    /// receivers, although some links have very asymmetric delays.
    /// 
    /// ```text
    /// [10 Nov 1995 11:33:25.125 UTC]       [10 Nov 1995 11:33:36.5 UTC]
    /// n                 SR(n)              A=b710:8000 (46864.500 s)
    /// ---------------------------------------------------------------->
    ///                    v                 ^
    /// ntp_sec =0xb44db705 v               ^ dlsr=0x0005:4000 (    5.250s)
    /// ntp_frac=0x20000000  v             ^  lsr =0xb705:2000 (46853.125s)
    ///   (3024992005.125 s)  v           ^
    /// r                      v         ^ RR(n)
    /// ---------------------------------------------------------------->
    ///                        |<-DLSR->|
    ///                        (5.250 s)
    /// 
    /// A     0xb710:8000 (46864.500 s)
    /// DLSR -0x0005:4000 (    5.250 s)
    /// LSR  -0xb705:2000 (46853.125 s)
    /// -------------------------------
    /// delay 0x0006:2000 (    6.125 s)
    /// ```
    pub dlsr: u32
}

impl TryFrom<&[u8]> for Source {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use rtcp::protocol::Source;
    ///
    /// let buffer = [
    ///     0x00, 0x00, 0x00, 0x01,
    ///     0x02,
    ///     0x00, 0x00, 0x03,
    ///     0x00, 0x00, 0x00, 0x04,
    ///     0x00, 0x00, 0x00, 0x05,
    ///     0x00, 0x00, 0x00, 0x06,
    ///     0x00, 0x00, 0x00, 0x07,
    /// ];
    ///
    /// let souce = Source::try_from(&buffer[..]).unwrap();
    /// assert_eq!(souce.identifier, 1);
    /// assert_eq!(souce.fraction_lost, 2);
    /// assert_eq!(souce.cnopl, 3);
    /// assert_eq!(souce.ehsnr, 4);
    /// assert_eq!(souce.interarrival_jitter, 5);
    /// assert_eq!(souce.lsr, 6);
    /// assert_eq!(souce.dlsr, 7);
    /// ```
    fn try_from(mut buf: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            identifier: buf.get_u32(),
            fraction_lost: buf.get_u8(),
            cnopl: buf.get_uint(3) as u32,
            ehsnr: buf.get_u32(),
            interarrival_jitter: buf.get_u32(),
            lsr: buf.get_u32(),
            dlsr: buf.get_u32(),
        })
    }
}

/// RTCP packet
pub enum Rtcp {
    SR(Sr),
    RR(Rr),
    // SDES(Sdes),
    // BYE(Bye),
    // APP(App),
}

impl Rtcp {
    /// packet type (PT): 8 bits
    /// Contains the constant 200 to identify this as an RTCP SR packet. 
    ///
    /// # Unit Test
    ///
    /// ```
    /// use rtcp::protocol::{
    ///     PacketKind,
    ///     Rtcp
    /// };
    ///
    /// let buffer = [0x80, 0xc8];
    /// let kind = Rtcp::packet_kind(&buffer).unwrap();
    /// assert_eq!(kind, PacketKind::SR);
    /// ```
    pub fn packet_kind(buf: &[u8]) -> Result<PacketKind> {
        assert!(buf.len() >= 2);
        Ok(PacketKind::try_from(buf[1])?)
    }
    
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
    /// use rtcp::protocol::{
    ///     PacketKind,
    ///     Rtcp
    /// };
    ///
    /// let buffer = [0x80, 0xc8, 0x00, 0x06];
    /// let len = Rtcp::peek_len(&buffer);
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
    /// assert!(rtcp.is_some());
    /// ```
    pub fn accept(&mut self) -> Result<Option<Rtcp>> {
        if self.buf.len() <= 4 {
            return Ok(None)
        }
        
        let kind = Rtcp::packet_kind(&self.buf[..])?;
        let size = Rtcp::peek_len(&self.buf[..]);
        if self.buf.len() < size {
            return Ok(None)
        }
        
        let body = &self.buf[..size];
        Ok(Some(match kind {
            PacketKind::SR => Rtcp::SR(Sr::try_from(body)?),
            PacketKind::RR => Rtcp::RR(Rr::try_from(body)?),
            _ => return Ok(None)
        }))
    }
}
