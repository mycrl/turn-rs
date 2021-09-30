use super::Source;
use anyhow::Result;
use std::convert::TryFrom;

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
pub struct ReceiverReport {
    pub sources: Option<Vec<Source>>,
}

impl TryFrom<&[u8]> for ReceiverReport {
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
    fn try_from(mut buf: &[u8]) -> Result<Self, Self::Error> {
        let sources = if buf.len() == 0 {
            Ok(Self { sources: None })
        }
        
        let step_num = buf.len() / 24;
        let mut list = Vec::with_capacity(step_num);
        for i in 0..step_num {
            let slice = &buf[i * 24..(i + 1) * 24];
            list.push(Source::try_from(slice)?);
        }

        Ok(Self {
            sources: Some(list)
        })
    }
}
