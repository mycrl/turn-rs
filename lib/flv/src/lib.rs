pub mod audio;
pub mod video;
mod exp_golomb;
mod sps;

use bytes::BytesMut;
use bytes::BufMut; 
use bytes::Bytes;

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

impl Default for Tag {
    fn default() -> Self {
        Self::Audio
    }
}

#[derive(Debug, Clone, Default)]
pub struct FrameRate {
    pub fixed: bool,
    pub fps: f64,
    pub fps_den: usize,
    pub fps_num: usize
}

#[derive(Debug, Clone, Default)]
pub struct Size {
    pub width: usize,
    pub height: usize
}

#[derive(Debug, Clone, Default)]
pub struct SPS {
    pub profile_string: String,  // baseline, high, high10, ...
    pub level_string: String,  // 3, 3.1, 4, 4.1, 5, 5.1, ...
    pub bit_depth: usize,  // 8bit, 10bit, ...
    pub ref_frames: usize,
    pub chroma_format: usize,  // 4:2:0, 4:2:2, ...
    pub chroma_format_string: String,
    pub frame_rate: FrameRate,
    pub sar_ratio: Size,
    pub codec_size: Size,
    pub present_size: Size
}

/// 媒体信息
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub tag: Tag,
    pub track_id: u8,
    pub timescale: u32,
    pub duration: u32,
    pub audio_sample_rate: u32,
    pub channel_count: u8,
    pub codec: String,
    pub original_codec: String,
    pub config: Bytes,
    pub ref_sample_duration: u32,
    pub codec_width: usize,
    pub codec_height: usize,
    pub present_width: usize,
    pub present_height: usize,
    pub profile: String,
    pub level: String,
    pub bit_depth: usize,
    pub chroma_format: usize,
    pub sar_ratio: Size,
    pub frame_rate: FrameRate,
    pub avcc: Bytes
}

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
