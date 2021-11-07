use std::convert::TryFrom;
use bytes::Buf;
use super::{
    Source,
    PADDING_MASK,
    RC_MASK
};

/// # RR: Receiver Report RTCP Packet
///
/// ```text
///        0                   1                   2                   3
///        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// header |V=2|P|    RC   |   PT=RR=201   |             length            |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                     SSRC of packet sender                     |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// report |                 SSRC_1 (SSRC of first source)                 |
/// block  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   1    | fraction lost |       cumulative number of packets lost       |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |           extended highest sequence number received           |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                      interarrival jitter                      |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                         last SR (LSR)                         |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                   delay since last SR (DLSR)                  |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// report |                 SSRC_2 (SSRC of second source)                |
/// block  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   2    :                               ...                             :
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
///        |                  profile-specific extensions                  |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
/// 
/// The format of the receiver report (RR) packet is the same as that of
/// the SR packet except that the packet type field contains the constant
/// 201 and the five words of sender information are omitted (these are
/// the NTP and RTP timestamps and sender's packet and octet counts).
/// The remaining fields have the same meaning as for the SR packet.
/// 
/// An empty RR packet (RC = 0) MUST be put at the head of a compound
/// RTCP packet when there is no data transmission or reception to
/// report.
pub struct Rr {
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
    /// SSRC: 32 bits
    /// The synchronization source identifier for the originator of this
    /// SR packet.
    pub ssrc: u32,
    pub sources: Option<Vec<Source>>,
}

impl TryFrom<&[u8]> for Rr {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use rtcp::protocol::Rr;
    ///
    /// let buffer = [
    ///     0x82, 0xc9, 0x00, 0x0d, 0x0a, 0xc8, 0x30, 0xb2, 
    ///     0x02, 0xe8, 0x09, 0xbe, 0x64, 0x84, 0xbb, 0xfc,
    ///     0x4a, 0xb7, 0x8d, 0x89, 0x0d, 0x3a, 0xc0, 0xb4,
    ///     0xc7, 0x6f, 0x96, 0x86, 0x87, 0x69, 0x4f, 0x2e,
    ///     0x4a, 0xac, 0x8c, 0x95, 0xf8, 0x4e, 0x95, 0x40,
    ///     0xef, 0xcf, 0x89, 0x2d, 0x2a, 0xfe, 0x42, 0x7a,
    ///     0x18, 0xe0, 0xe3, 0x26, 0xa0, 0xa2, 0xa0, 0x15
    /// ];
    ///
    /// let rr = Rr::try_from(&buffer[..]).unwrap();
    /// assert_eq!(rr.padding, false);
    /// assert_eq!(rr.rc, 2);
    /// assert_eq!(rr.ssrc, 0x0ac830b2);
    /// assert_eq!(rr.sources.unwrap().len(), 2);
    /// ```
    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {assert!(buf.len() >= 4);
        assert!(buf.len() >= 4);

        let rc = buf[0] & RC_MASK;
        let padding = ((buf[0] & PADDING_MASK) >> 5) == 1;
        
        let pd_size = if padding {
            buf[buf.len() - 1] as usize
        } else {
            0
        };

        let pl_size = buf.len() - pd_size;
        let mut body = &buf[4..pl_size];

        let ssrc = body.get_u32();
        let remaining = body.remaining();
        if remaining == 0 {
            return Ok(Self { 
                sources: None,
                padding,
                ssrc,
                rc
            })
        }
        
        let step_num = remaining / 24;
        let mut list = Vec::with_capacity(step_num);
        for i in 0..step_num {
            let slice = &body[i * 24..(i + 1) * 24];
            list.push(Source::try_from(slice)?);
        }

        Ok(Self {
            sources: Some(list),
            padding,
            ssrc,
            rc
        })
    }
}
