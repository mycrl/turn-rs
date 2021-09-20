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

pub mod extensions;

use extensions::Extensions;
use std::convert::TryFrom;
use anyhow::ensure;
use bytes::{
    BytesMut,
    BufMut,
    Buf
};

const MARKER_MASK: u8 = 0b10000000;
const VERSION_MASK: u8 = 0b11000000;
const PADDING_MASK: u8 = 0b00100000;
const EXTENSION_MASK: u8 = 0b00010000;
const CSRC_COUNT_MASK: u8 = 0b00001111;
const PAYLOAD_KIND_MASK: u8 = 0b01111111;

const LE_VERSION_MASK: u8 = !VERSION_MASK;
const LE_CSRC_COUNT_MASK: u8 = !CSRC_COUNT_MASK;
const LE_PAYLOAD_KIND_MASK: u8 = !PAYLOAD_KIND_MASK;

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
    /// The interpretation of the marker is defined by a profile.  It is
    /// intended to allow significant events such as frame boundaries to
    /// be marked in the packet stream.  A profile MAY define additional
    /// marker bits or specify that there is no marker bit by changing the
    /// number of bits in the payload type field.
    pub marker: bool,
    /// This field identifies the format of the RTP payload and determines
    /// its interpretation by the application.  A profile MAY specify a
    /// default static mapping of payload type codes to payload formats.
    /// Additional payload type codes MAY be defined dynamically through
    /// non-RTP means.  A set of default mappings for audio and video is 
    /// specified in the companion RFC 3551 
    /// [1](https://tools.ietf.org/html/rfc3551). An RTP source MAY change 
    /// the payload type during a session, but this field SHOULD NOT be used 
    /// for multiplexing separate media streams.
    /// 
    /// A receiver MUST ignore packets with payload types that it does not
    /// understand.
    pub kind: u8,
    /// The sequence number increments by one for each RTP data packet
    /// sent, and may be used by the receiver to detect packet loss and to
    /// restore packet sequence.  The initial value of the sequence number
    /// SHOULD be random (unpredictable) to make known-plaintext attacks
    /// on encryption more difficult, even if the source itself does not
    /// encrypt according to the method in 
    /// [Section 9.1](https://tools.ietf.org/html/rfc3550#section-9.1), 
    /// because the packets may flow through a translator that does.  
    /// Techniques for choosing unpredictable numbers are discussed in 
    /// [17](https://tools.ietf.org/html/rfc3550#ref-17).
    pub sequence_number: u16,
    /// The timestamp reflects the sampling instant of the first octet in
    /// the RTP data packet.
    pub timestamp: u32,
    /// The SSRC field identifies the synchronization source.  This
    /// identifier SHOULD be chosen randomly, with the intent that no two
    /// synchronization sources within the same RTP session will have the
    /// same SSRC identifier.  An example algorithm for generating a
    /// random identifier is presented in 
    /// [Appendix A.6](https://tools.ietf.org/html/rfc3550#appendix-A.6).  
    /// Although the probability of multiple sources choosing the same 
    /// identifier is low, all RTP implementations must be prepared to 
    /// detect and resolve collisions.  
    /// [Section 8](https://tools.ietf.org/html/rfc3550#section-8) 
    /// describes the probability of collision along with a mechanism for 
    /// resolving collisions and detecting RTP-level forwarding loops based 
    /// on the uniqueness of the SSRC identifier.  If a source changes its 
    /// source transport address, it must also choose a new SSRC identifier 
    /// to avoid being interpreted as a looped source.
    pub ssrc: u32,
    /// The CSRC list identifies the contributing sources for the payload
    /// contained in this packet.  The number of identifiers is given by
    /// the CC field.  If there are more than 15 contributing sources,
    /// only 15 can be identified.  CSRC identifiers are inserted by
    /// mixer, using the SSRC identifiers of contributing sources.  
    /// For example, for audio packets the SSRC identifiers of all sources 
    /// that were mixed together to create a packet are listed, allowing 
    /// correct talker indication at the receiver.
    pub csrc: Option<Vec<u32>>,
    /// If the extension bit is set, the fixed header MUST be followed by
    /// exactly one header extension.
    pub extensions: Option<Extensions<'a>>,
    /// If the padding bit is set, the packet contains one or more
    /// additional padding octets at the end which are not part of the
    /// payload.  The last octet of the padding contains a count of how
    /// many padding octets should be ignored, including itself.  Padding
    /// may be needed by some encryption algorithms with fixed block sizes
    /// or for carrying several RTP packets in a lower-layer protocol data
    /// unit.
    pub padding: Option<&'a [u8]>,
    pub payload: Option<&'a [u8]>,
}

impl<'a> Rtp<'a> {
    /// # Unit Test
    ///
    /// ```
    /// use rtp::Rtp;
    /// use bytes::BytesMut;
    /// use rtp::extensions::*;
    ///
    /// let buffer = [
    ///     0xB0, 0x72, 0x04, 0xf1, 0xf8, 0x87, 0x3f, 0xad, 0x67, 0xfe,
    ///     0x9d, 0xfc, 0xbe, 0xde, 0x00, 0x01, 0x22, 0xaa, 0x36, 0x3f,
    ///     0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x05,
    /// ];
    /// 
    /// let extensions = Some(Extensions(vec![
    ///     Extension {
    ///         data: &[0xaa, 0x36, 0x3f],
    ///         kind: 2,
    ///     }
    /// ]));
    /// 
    /// let rtp = Rtp {
    ///     extensions,
    ///     marker: false,
    ///     kind: 114,
    ///     sequence_number: 1265,
    ///     timestamp: 4169613229,
    ///     ssrc: 1744739836,
    ///     payload: Some(&[0x00, 0x00, 0x00, 0x00, 0x00]),
    ///     padding: Some(&[0x01, 0x01, 0x01, 0x01]),
    ///     csrc: None,
    /// };
    /// 
    /// let mut writer = BytesMut::new();
    /// rtp.into(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    #[rustfmt::skip]
    pub fn into(self, buf: &mut BytesMut) {
        let is_pd = self.padding.is_some();
        let is_es = self.extensions.is_some();
        let cc = match &self.csrc {
            Some(c) => c.len() as u8,
            None => 0,
        };
        
        let mut basic = [0u8; 2];
        basic[0] = (basic[0] & LE_VERSION_MASK) | (2 << 6);
        basic[0] = if is_pd { basic[0] | 1 << 5 } else { basic[0] & !(1 << 5) };
        basic[0] = if is_es { basic[0] | 1 << 4 } else { basic[0] & !(1 << 4) };
        basic[0] = (basic[0] & LE_CSRC_COUNT_MASK) | (cc << 0);
        basic[1] = if self.marker { basic[1] | 1 << 4 } else { basic[1] & !(1 << 4) };
        basic[1] = (basic[1] & LE_PAYLOAD_KIND_MASK) | (self.kind << 0);
        
        buf.put(&basic[..]);
        buf.put_u16(self.sequence_number);
        buf.put_u32(self.timestamp);
        buf.put_u32(self.ssrc);
        
        if let Some(csrc) = self.csrc {
            for item in csrc {
                buf.put_u32(item);
            }
        }

        if let Some(e) = self.extensions {
            e.into(buf);
        }

        if let Some(p) = self.payload {
            buf.put(p);
        }
        
        if let Some(p) = self.padding {
            buf.put(p);
            buf.put_u8((p.len() + 1) as u8);
        }
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
    ///     0xB0, 0x72, 0x04, 0xf1, 0xf8, 0x87, 0x3f, 0xad, 0x67, 0xfe,
    ///     0x9d, 0xfc, 0xbe, 0xde, 0x00, 0x01, 0x22, 0xaa, 0x36, 0x3f,
    ///     0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x05,
    /// ];
    /// 
    /// let rtp = Rtp::try_from(&buffer[..]).unwrap();
    /// assert_eq!(rtp.marker, false);
    /// assert_eq!(rtp.kind, 114);
    /// assert_eq!(rtp.sequence_number, 1265);
    /// assert_eq!(rtp.timestamp, 4169613229);
    /// assert_eq!(rtp.ssrc, 1744739836);
    /// assert!(rtp.csrc.is_none());
    /// assert!(rtp.padding.is_some());
    /// assert!(rtp.extensions.is_some());
    ///
    /// let extensions = rtp.extensions.unwrap();
    /// assert_eq!(extensions.0.len(), 1);
    /// assert_eq!(extensions.0[0].kind, 2);
    /// 
    /// let payload = rtp.payload.unwrap();
    /// assert_eq!(payload, &[0x00, 0x00, 0x00, 0x00, 0x00]);
    /// 
    /// let padding = rtp.padding.unwrap();
    /// assert_eq!(padding, &[0x01, 0x01, 0x01, 0x01]);
    /// ```
    #[rustfmt::skip]
    fn try_from(mut buf: &'a [u8]) -> Result<Self, Self::Error> {
        ensure!(buf.len() >= 12, "buf len < 12");
        
        // lock rtp version in rfc 3550
        let version = (buf[0] & VERSION_MASK) >> 6;
        ensure!(version == 2, "rtp version is not rfc3550!");
        
        let is_padding = ((buf[0] & PADDING_MASK) >> 5) == 1;
        let is_extension = ((buf[0] & EXTENSION_MASK) >> 4) == 1;
        let csrc_count = (buf[0] & CSRC_COUNT_MASK) as usize;
        let marker = ((buf[1] & MARKER_MASK) >> 7) == 1;
        let kind = buf[1] & PAYLOAD_KIND_MASK;
        buf.advance(2);
        
        let size = 10 + (csrc_count * 4);
        ensure!(buf.len() >= size, "buf len is too short!");
        
        let sequence_number = buf.get_u16();
        let timestamp = buf.get_u32();
        let ssrc = buf.get_u32();
        
        let csrc = if csrc_count > 0 {
            let c = (0..csrc_count)
                .map(|_| buf.get_u32())
                .collect::<Vec<u32>>();
            Some(c)
        } else {
            None  
        };

        let extensions = if is_extension {
            let es = Extensions::try_from(&buf[..])?;
            buf.advance(es.len());
            Some(es)
        } else {
            None 
        };

        let pd_size = if is_padding {
            buf[buf.len() - 1] as usize
        } else {
            0
        };

        let pl_size = buf.len() - pd_size;
        let payload = if pl_size > 0 {
            let p = &buf[..pl_size];
            buf.advance(pl_size);
            Some(p)
        } else {
            None
        };
        
        let padding = if pd_size > 0 {
            Some(&buf[..buf.len() - 1])
        } else {
            None 
        };

        Ok(Self {
            kind,
            csrc,
            ssrc,
            marker,
            padding,
            payload,
            extensions,
            timestamp,
            sequence_number,
        })
    }
}
