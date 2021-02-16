use std::convert::TryFrom;
use anyhow::ensure;
use bytes::{
    BytesMut,
    // BufMut,
    Bytes,
    Buf
};

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
    /// This field identifies the version of RTP.  The version defined by
    /// this specification is two (2).  (The value 1 is used by the first
    /// draft version of RTP and the value 0 is used by the protocol
    /// initially implemented in the "vat" audio tool.)
    pub version: u8,
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
    /// The CSRC count contains the number of CSRC identifiers that follow
    /// the fixed header.
    pub csrc_count: u8,
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
    pub fn into(self, _buf: &mut BytesMut) {
        
    }
}

impl<'a> TryFrom<&'a mut Bytes> for Header {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use bytes::Bytes;
    /// use rtp::header::Header;
    /// use std::convert::TryFrom;
    ///
    /// let mut buffer = Bytes::from_static(&[
    ///     0x90, 0x72, 0x04, 0xf1, 0xf8, 0x87, 0x3f, 0xad, 0x67, 0xfe,
    ///     0x9d, 0xfc
    /// ]);
    /// 
    /// let header = Header::try_from(&mut buffer).unwrap();
    /// assert_eq!(header.version, 2);
    /// assert_eq!(header.padding, false);
    /// assert_eq!(header.extension, true);
    /// assert_eq!(header.csrc_count, 0);
    /// assert_eq!(header.marker, false);
    /// assert_eq!(header.payload_kind, 114);
    /// assert_eq!(header.sequence_number, 1265);
    /// assert_eq!(header.timestamp, 4169613229);
    /// assert_eq!(header.ssrc, 1744739836);
    /// assert_eq!(header.csrc_list.len(), 0);
    /// ```
    #[rustfmt::skip]
    fn try_from(buf: &'a mut Bytes) -> Result<Self, Self::Error> {
        ensure!(buf.len() >= 12, "buf len < 12");
        
        // create bit reader,
        // and get basic header attribute.
        let version = buf[0] >> 6;
        let padding = ((buf[0] >> 5) & 1) == 1;
        let extension = ((buf[0] >> 4) & 1) == 1;
        let csrc_count = buf[0] & 15;
        let marker = (buf[1] >> 7) == 1;
        let payload_kind = buf[1] & 0x7F;
        
        // if the buf size is not long 
        // enough to continue, return a error.
        let size = 10 + (csrc_count as usize * 4);
        ensure!(buf.len() >= size, "buf len is too short");
        buf.advance(2);
        
        // get header attribute.
        let sequence_number = buf.get_u16();
        let timestamp = buf.get_u32();
        let ssrc = buf.get_u32();
        
        // get csrc list from csrc count attribute.
        let csrc_list = (0..csrc_count as usize)
            .map(|_| buf.get_u32())
            .collect::<Vec<u32>>();
        
        Ok(Self {
            ssrc,
            marker,
            version,
            padding,
            csrc_list,
            extension,
            timestamp,
            csrc_count,
            payload_kind,
            sequence_number,
        })
    }
}