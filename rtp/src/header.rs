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

/// RTP Header.
///
/// ### RTP Fixed Header Fields
/// 
/// ```bash
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |V=2|P|X|  CC   |M|     PT      |       sequence number         |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |                           timestamp                           |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |           synchronization source (SSRC) identifier            |
///  +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
///  |            contributing source (CSRC) identifiers             |
///  |                             ....                              |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Debug, Clone)]
pub struct Header {
    /// If the padding bit is set, the packet contains one or more
    /// additional padding octets at the end which are not part of the
    /// payload.  The last octet of the padding contains a count of how
    /// many padding octets should be ignored, including itself.  Padding
    /// may be needed by some encryption algorithms with fixed block sizes
    /// or for carrying several RTP packets in a lower-layer protocol data
    /// unit.
    pub padding: bool,
    /// If the extension bit is set, the fixed header MUST be followed by
    /// exactly one header extension.
    pub extension: bool,
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
    pub payload_kind: u8,
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
    pub csrc_list: Vec<u32>,
}

impl Header {
    /// # Unit Test
    ///
    /// ```
    /// use rtp::header::Header;
    /// use std::convert::TryFrom;
    ///
    /// let mut buffer = [
    ///     0x90, 0x72, 0x04, 0xf1, 0xf8, 0x87, 0x3f, 0xad, 0x67, 0xfe,
    ///     0x9d, 0xfc
    /// ];
    /// 
    /// let header = Header::try_from(&buffer[..]).unwrap();
    /// assert_eq!(header.len(), 12);
    /// ```
    pub fn len(&self) -> usize {
        12 + (self.csrc_list.len() * 4)
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
    /// header.into(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    #[rustfmt::skip]
    pub fn into(self, buf: &mut BytesMut) {
        let mut basic = [0u8; 2];
        
        basic[0] = (basic[0] & LE_VERSION_MASK) | (2 << 6);
        basic[0] = if self.padding { basic[0] | 1 << 5 } else { basic[0] & !(1 << 5) };
        basic[0] = if self.extension { basic[0] | 1 << 4 } else { basic[0] & !(1 << 4) };
        basic[0] = (basic[0] & LE_CSRC_COUNT_MASK) | ((self.csrc_list.len() as u8) << 0);
        
        basic[1] = if self.marker { basic[1] | 1 << 4 } else { basic[1] & !(1 << 4) };
        basic[1] = (basic[1] & LE_PAYLOAD_KIND_MASK) | (self.payload_kind << 0);
        
        buf.put(&basic[..]);
        buf.put_u16(self.sequence_number);
        buf.put_u32(self.timestamp);
        buf.put_u32(self.ssrc);
        
        for item in self.csrc_list {
            buf.put_u32(item);
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for Header {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use rtp::header::Header;
    /// use std::convert::TryFrom;
    ///
    /// let mut buffer = [
    ///     0x90, 0x72, 0x04, 0xf1, 0xf8, 0x87, 0x3f, 0xad, 0x67, 0xfe,
    ///     0x9d, 0xfc
    /// ];
    /// 
    /// let header = Header::try_from(&buffer[..]).unwrap();
    /// assert_eq!(header.padding, false);
    /// assert_eq!(header.extension, true);
    /// assert_eq!(header.marker, false);
    /// assert_eq!(header.payload_kind, 114);
    /// assert_eq!(header.sequence_number, 1265);
    /// assert_eq!(header.timestamp, 4169613229);
    /// assert_eq!(header.ssrc, 1744739836);
    /// assert_eq!(header.csrc_list.len(), 0);
    /// ```
    #[rustfmt::skip]
    fn try_from(mut buf: &'a [u8]) -> Result<Self, Self::Error> {
        ensure!(buf.len() >= 12, "buf len < 12");
        
        // lock rtp version in rfc 3550
        let version = (buf[0] & VERSION_MASK) >> 6;
        ensure!(version == 2, "rtp version is not rfc3550!");
        
        let padding = ((buf[0] & PADDING_MASK) >> 5) == 1;
        let extension = ((buf[0] & EXTENSION_MASK) >> 4) == 1;
        let csrc_count = (buf[0] & CSRC_COUNT_MASK) as usize;
        let marker = ((buf[1] & MARKER_MASK) >> 7) == 1;
        let payload_kind = buf[1] & PAYLOAD_KIND_MASK;
        buf.advance(2);
        
        let size = 10 + (csrc_count * 4);
        ensure!(buf.len() >= size, "buf len is too short!");
        
        let sequence_number = buf.get_u16();
        let timestamp = buf.get_u32();
        let ssrc = buf.get_u32();
        
        let csrc_list = (0..csrc_count)
            .map(|_| buf.get_u32())
            .collect::<Vec<u32>>();
        
        Ok(Self {
            ssrc,
            marker,
            padding,
            csrc_list,
            extension,
            timestamp,
            payload_kind,
            sequence_number,
        })
    }
}
