//! ## RTP: A Transport Protocol for Real-Time Applications
//!
//! This project specifies the real-time transport protocol (RTP),
//! which provides end-to-end delivery services for data with real-time
//! characteristics, such as interactive audio and video.  Those services
//! include payload type identification, sequence numbering, timestamping
//! and delivery monitoring.  Applications typically run RTP on top of
//! UDP to make use of its multiplexing and checksum services; both
//! protocols contribute parts of the transport protocol functionality.
//! However, RTP may be used with other suitable underlying network or
//! transport protocols. RTP supports data transfer to multiple 
//! destinations using multicast distribution if provided by the
//! underlying network.
//! 
//! Note that RTP itself does not provide any mechanism to ensure timely
//! delivery or provide other quality-of-service guarantees, but relies
//! on lower-layer services to do so.  It does not guarantee delivery or
//! prevent out-of-order delivery, nor does it assume that the underlying
//! network is reliable and delivers packets in sequence.  The sequence
//! numbers included in RTP allow the receiver to reconstruct the
//! sender's packet sequence, but sequence numbers might also be used to
//! determine the proper location of a packet, for example in video
//! decoding, without necessarily decoding packets in sequence.
//! 
//! While RTP is primarily designed to satisfy the needs of multi-
//! participant multimedia conferences, it is not limited to that
//! particular application.  Storage of continuous data, interactive
//! distributed simulation, active badge, and control and measurement
//! applications may also find RTP applicable.
//!

pub mod header;
pub mod extensions;

use header::Header;
use extensions::Extensions;
use std::convert::TryFrom;
use bytes::{
    BytesMut,
    BufMut
};

/// ### RTP Data Transfer Protocol
///
/// ```bash
///   0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |V=2|P|X|  CC   |M|     PT      |       sequence number         |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                           timestamp                           |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |           synchronization source (SSRC) identifier            |
/// +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// |            contributing source (CSRC) identifiers             |
/// |                             ....                              |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Debug, Clone)]
pub struct Rtp<'a> {
    pub header: Header,
    pub extensions: Option<Extensions<'a>>,
    pub payload: &'a [u8],
}

impl<'a> Rtp<'a> {
    /// # Unit Test
    ///
    /// ```
    /// use rtp::Rtp;
    /// use bytes::BytesMut;
    /// use rtp::header::Header;
    /// use rtp::extensions::*;
    ///
    /// let buffer = [
    ///     0x90, 0x72, 0x04, 0xf1, 0xf8, 0x87, 0x3f, 0xad, 0x67, 0xfe,
    ///     0x9d, 0xfc, 0xbe, 0xde, 0x00, 0x01, 0x22, 0xaa, 0x36, 0x3f
    /// ];
    ///
    /// let mut writer = BytesMut::new();
    /// let header = Header {
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
    /// let extensions = Some(Extensions(vec![
    ///     Extension {
    ///         data: &[0xaa, 0x36, 0x3f],
    ///         kind: 2,
    ///     }
    /// ]));
    /// 
    /// let rtp = Rtp {
    ///     header,
    ///     extensions,
    ///     payload: &[]
    /// };
    /// 
    /// rtp.encode(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    pub fn encode(self, buf: &mut BytesMut) {
        self.header.into(buf);
        if let Some(e) = self.extensions {
            e.into(buf);
        }

        buf.put(self.payload);
    }
}

impl<'a> TryFrom<&'a [u8]> for Rtp<'a> {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use rtp::Rtp;
    /// use std::convert::TryFrom;
    ///
    /// let buffer = [
    ///     0x90, 0x72, 0x04, 0xf1, 0xf8, 0x87, 0x3f, 0xad, 0x67, 0xfe,
    ///     0x9d, 0xfc, 0xbe, 0xde, 0x00, 0x01, 0x22, 0xaa, 0x36, 0x3f
    /// ];
    /// 
    /// let rtp = Rtp::try_from(&buffer[..]).unwrap();
    /// assert_eq!(rtp.header.padding, false);
    /// assert_eq!(rtp.header.extension, true);
    /// assert_eq!(rtp.header.marker, false);
    /// assert_eq!(rtp.header.payload_kind, 114);
    /// assert_eq!(rtp.header.sequence_number, 1265);
    /// assert_eq!(rtp.header.timestamp, 4169613229);
    /// assert_eq!(rtp.header.ssrc, 1744739836);
    /// assert_eq!(rtp.header.csrc_list.len(), 0);
    ///
    /// let extensions = rtp.extensions.unwrap();
    /// assert_eq!(extensions.0.len(), 1);
    /// assert_eq!(extensions.0[0].kind, 2);
    /// ```
    #[rustfmt::skip]
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        let mut offset = 0;

        let header = Header::try_from(buf)?;
        offset += header.len();

        let extensions = if header.extension {
            let es = Extensions::try_from(&buf[offset..])?;
            offset += es.len();
            Some(es)
        } else {
            None 
        };

        Ok(Self {
            header,
            extensions,
            payload: &buf[offset..],
        })
    }
}
