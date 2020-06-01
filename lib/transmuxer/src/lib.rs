mod flv;
mod fmp4;
mod aac;
mod avc;

pub use flv::Flv;
use bytes::{BytesMut, Bytes};
use transport::Flag;

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

/// 视频规格
#[derive(Debug, Clone, Default)]
pub struct FrameRate {
    pub fixed: bool,
    pub fps: f64,
    pub fps_den: u32,
    pub fps_num: u32
}

/// 视频尺寸
#[derive(Debug, Clone, Default)]
pub struct Size {
    pub width: u32,
    pub height: u32
}

/// H264 SPS信息
#[derive(Debug, Clone, Default)]
pub struct SPS {
    pub profile_string: String,  // baseline, high, high10, ...
    pub level_string: String,  // 3, 3.1, 4, 4.1, 5, 5.1, ...
    pub bit_depth: u32,  // 8bit, 10bit, ...
    pub ref_frames: u32,
    pub chroma_format: u32,  // 4:2:0, 4:2:2, ...
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
    pub codec_width: u32,
    pub codec_height: u32,
    pub present_width: u32,
    pub present_height: u32,
    pub profile: String,
    pub level: String,
    pub bit_depth: u32,
    pub chroma_format: u32,
    pub sar_ratio: Size,
    pub frame_rate: FrameRate,
    pub avcc: Bytes
}

/// 视频样本
#[derive(Debug, Clone)]
pub struct VideoSample {
    pub units: Vec<(usize, BytesMut)>,
    pub dts: u32,
    pub cts: u32,
    pub pts: u32
}

/// 解码器解码结果
pub enum DecoderResult {
    AudioMetadata(Metadata),
    AudioTrack((u32, BytesMut)),
    VideoMetadata(Metadata),
    VideoTrack(VideoSample)
}

impl Default for Tag {
    fn default() -> Self {
        Self::Audio
    }
}

pub fn to_fmp4(flag: Flag, data: BytesMut, timestamp: u32) {
    if let Flag::FlvAudio = flag {
        if let DecoderResult::AudioMetadata(meta) = aac::decoder(data, timestamp) {
            
        }
    }
}
