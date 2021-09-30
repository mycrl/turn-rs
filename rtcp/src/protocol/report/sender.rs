use super::Source;
use std::convert::TryFrom;
use anyhow::Result;
use bytes::Buf;

/// # SR: Sender Report RTCP Packet
///
/// ```text
///        0                   1                   2                   3
///        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// header |V=2|P|    RC   |   PT=SR=200   |             length            |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                         SSRC of sender                        |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// sender |              NTP timestamp, most significant word             |
/// info   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |             NTP timestamp, least significant word             |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                         RTP timestamp                         |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                     sender's packet count                     |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                      sender's octet count                     |
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
/// The sender report packet consists of three sections, possibly
/// followed by a fourth profile-specific extension section if defined.
/// The first section, the header, is 8 octets long.  The fields have the
/// following meaning:
pub struct SenderReport {
    /// NTP timestamp: 64 bits
    /// Indicates the wallclock time when this report was sent so 
    /// that it may be used in combination with timestamps returned 
    /// in reception reports from other receivers to measure
    /// round-trip propagation to those receivers.  Receivers should
    /// expect that the measurement accuracy of the timestamp may be
    /// limited to far less than the resolution of the NTP timestamp.  The
    /// measurement uncertainty of the timestamp is not indicated as it
    /// may not be known.  On a system that has no notion of wallclock
    /// time but does have some system-specific clock such as "system
    /// uptime", a sender MAY use that clock as a reference to calculate
    /// relative NTP timestamps.  It is important to choose a commonly
    /// used clock so that if separate implementations are used to produce
    /// the individual streams of a multimedia session, all
    /// implementations will use the same clock.  Until the year 2036,
    /// relative and absolute timestamps will differ in the high bit so
    /// (invalid) comparisons will show a large difference; by then one
    /// hopes relative timestamps will no longer be needed.  A sender that
    /// has no notion of wallclock or elapsed time MAY set the NTP
    /// timestamp to zero.
    pub ntp_time: u64,
    /// RTP timestamp: 32 bits
    /// Corresponds to the same time as the NTP timestamp (above), but in
    /// the same units and with the same random offset as the RTP
    /// timestamps in data packets.  This correspondence may be used for
    /// intra- and inter-media synchronization for sources whose NTP
    /// timestamps are synchronized, and may be used by media-independent
    /// receivers to estimate the nominal RTP clock frequency.  Note that
    /// in most cases this timestamp will not be equal to the RTP
    /// timestamp in any adjacent data packet.  Rather, it MUST be
    /// calculated from the corresponding NTP timestamp using the
    /// relationship between the RTP timestamp counter and real time as
    /// maintained by periodically checking the wallclock time at a
    /// sampling instant.
    pub rtp_time: u32,
    /// sender's packet count: 32 bits
    /// The total number of RTP data packets transmitted by the sender
    /// since starting transmission up until the time this SR packet was
    /// generated.  The count SHOULD be reset if the sender changes its
    /// SSRC identifier.
    pub sender_packet_count: u32,
    /// sender's octet count: 32 bits
    /// The total number of payload octets (i.e., not including header or
    /// padding) transmitted in RTP data packets by the sender since
    /// starting transmission up until the time this SR packet was
    /// generated.  The count SHOULD be reset if the sender changes its
    /// SSRC identifier.  This field can be used to estimate the average
    /// payload data rate.
    pub sender_octet_count: u32,
    /// The third section contains zero or more reception report blocks
    /// depending on the number of other sources heard by this sender since
    /// the last report.  Each reception report block conveys statistics on
    /// the reception of RTP packets from a single synchronization source.
    /// Receivers SHOULD NOT carry over statistics when a source changes its
    /// SSRC identifier due to a collision.
    pub sources: Option<Vec<Source>>,
}

impl TryFrom<&[u8]> for SenderReport {
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
        let ntp_time = buf.get_u64();
        let rtp_time = buf.get_u32();
        let sender_packet_count = buf.get_u32();
        let sender_octet_count = buf.get_u32();

        let remaining = buf.remaining();
        let sources = if remaining == 0 {
            None
        } else {
            let step_num = remaining / 24;
            let mut list = Vec::with_capacity(step_num);
            for i in 0..step_num {
                list.push(Source::try_from(&buf[i * 24..(i + 1) * 24])?);
            }

            Some(list)
        };

        Ok(Self {
            sender_packet_count,
            sender_octet_count,
            ntp_time,
            rtp_time,
            sources
        })
    }
}
