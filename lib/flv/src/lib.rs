use bytes::{BytesMut, Bytes, BufMut};

/// Flv header type.
///
/// audio or video or all.
#[derive(Debug)]
pub enum Header {
    Audio,
    Video,
    Full
}

/// Flv tag type
///
/// script(amf),
/// audio tag,
/// video tag.
#[derive(Debug, Clone)]
pub enum Tag {
    Script,
    Audio,
    Video
}

pub struct Flv;
impl Flv {
    /// Create FLV frame
    ///
    /// Timestamp and TimestampExtended form the 
    /// PTS information of this TAG packet data, 
    /// PTS = Timestamp | TimestampExtended << 24.
    #[rustfmt::skip]
    pub fn encode_tag(data: &[u8], tag: Tag, timestamp: u32) -> Bytes {
        let mut buffer = BytesMut::new();
        let data_size = data.len();
        let size = data_size + 11;
        let flag = match tag {
            Tag::Script => 0x12,
            Tag::Audio => 0x08,
            Tag::Video => 0x09
        };

        // tag type
        // body size
        buffer.put_u8(flag);
        buffer.put_uint(data_size as u64, 3);

        // timestamp
        buffer.extend_from_slice(&[
            ((timestamp >> 16) & 0xff) as u8,
            ((timestamp >> 8) & 0xff) as u8,
            (timestamp & 0xff) as u8,
            ((timestamp >> 24) & 0xff) as u8
        ]);

        // fixed zero
        // media body
        // tag size
        buffer.put_uint(0, 3);
        buffer.extend_from_slice(data);
        buffer.put_u32(size as u32);

        buffer.freeze()
    }

    /// Create FLV header
    ///
    /// Generally, the first 13 bytes of FLV 
    /// (flv header + PreviousTagSize0) are 
    /// exactly the same.
    #[rustfmt::skip]
    pub fn encode_header(head: Header) -> Bytes {
        let flag = match head {
            Header::Audio => 0x04,
            Header::Video => 0x01,
            Header::Full => 0x05
        };

        BytesMut::from([

            // "FLV"
            0x46, 
            0x4c,
            0x56, 

            // version
            0x01,

            // flag
            flag,

            // size
            0x00, 
            0x00, 
            0x00, 
            0x09,

            // size
            0, 0, 0, 0
        ].to_vec().as_slice())
            .freeze()
    }
}
