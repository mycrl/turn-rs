use std::convert::TryFrom;
use anyhow::Result;
use bytes::Buf;

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
    pub dlsr: u32
}

impl TryFrom<&[u8]> for Source {
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
