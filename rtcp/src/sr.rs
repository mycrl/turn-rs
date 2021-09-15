use std::convert::TryFrom;
use anyhow::Result;
use bytes::{
    BytesMut,
    Buf
};

pub struct Sr {
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
    pub ntp_timestamp: u64,
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
    pub rtp_timestamp: u32,
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
}

impl TryFrom<&mut BytesMut> for Sr {
    type Error = anyhow::Error;
    fn try_from(buf: &mut BytesMut) -> Result<Self, Self::Error> {
        let ntp_timestamp = buf.get_u64();
        let rtp_timestamp = buf.get_u32();
        let sender_packet_count = buf.get_u32();
        let sender_octet_count = buf.get_u32();
        Ok(Self {
            sender_packet_count,
            sender_octet_count,
            ntp_timestamp,
            rtp_timestamp,
        })
    }
}
