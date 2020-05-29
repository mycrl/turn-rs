mod exp_golomb;
mod sps;

use bytes::BytesMut;
use bytes::BufMut; 
use bytes::Bytes;
use bytes::Buf;

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

pub struct VideoSample {
    units: Vec<(usize, BytesMut)>,
    dts: u32,
    cts: u32,
    pts: u32
}

pub enum DecoderResult {
    Metadata(Metadata),
    AudioTrack((u32, BytesMut)),
    VideoTrack(VideoSample)
}

impl Default for Tag {
    fn default() -> Self {
        Self::Audio
    }
}

/// Flv处理
/// 
/// Flv编解码器
/// 打包flv header和tag，以及处理音频
/// 和视频帧，拆分出音视频信息和轨道.
pub struct Flv {
    nalu_length_size: usize
}

impl Flv {
    pub fn new() -> Self {
        Self {
            nalu_length_size: 4
        }
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

    /// 解析音频帧
    /// 
    /// 注意: 只支持AAC.
    /// 输入音频帧获得音频帧信息.
    #[rustfmt::skip]
    pub fn decoder_audio(mut data: BytesMut, timestamp: u32) -> DecoderResult {

        // 非关键帧
        // 非关键帧直接组成音频轨道
        if data[1] == 1 {
            data.advance(1);
            return DecoderResult::AudioTrack((timestamp, data));
        }

        // 关键帧
        // 解析出关键帧的媒体信息
        let mut meta = Metadata::default();
        let audio_object_type = data[2] >> 3;
        let sampling_index = ((data[2] & 0x07) << 1) | (data[3] >> 7);
        let sampling_frequence = match sampling_index {
            0 => 96000, 
            1 => 88200, 
            2 => 64000, 
            3 => 48000, 
            4 => 44100, 
            5 => 32000,
            6 => 24000, 
            7 => 22050, 
            8 => 16000, 
            9 => 12000, 
            10 => 11025, 
            11 => 8000, 
            _ => 7350
        };

        let channel_config = (data[3] & 0x78) >> 3;
        let mut extension_sampling = sampling_index;
        if sampling_index >= 6 {
            extension_sampling = sampling_index - 3;
        }

        let mut config = [0u8; 4];
        config[0] = audio_object_type << 3;
        config[0] |= (sampling_index & 0x0F) >> 1;
        config[1] = (sampling_index & 0x0F) << 7;
        config[1] |= (channel_config & 0x0F) << 3;
        config[1] |= (extension_sampling & 0x0F) >> 1;
        config[2] |= (extension_sampling & 0x01) << 7;
        config[2] |= 2 << 2;

        meta.tag = Tag::Audio;
        meta.track_id = 2;
        meta.duration = 0;
        meta.timescale = 1000;
        meta.audio_sample_rate = sampling_frequence;
        meta.channel_count = channel_config;
        meta.codec = "mp4a.40.5".to_string();
        meta.original_codec = "mp4a.40.5".to_string();
        meta.config = Bytes::from(config.to_vec());
        meta.ref_sample_duration = 1024 / sampling_frequence * 1000;
        DecoderResult::Metadata(meta)
    }

    /// 解析视频帧
    /// 
    /// 注意: 只支持H264
    #[rustfmt::skip]
    pub fn decoder_video(&mut self, mut data: BytesMut, timestamp: u32) -> DecoderResult {
        let avcc = data.clone();
        let spec = data.get_u8();
        let frame_type = (spec & 240) >> 4;
        let codec_id = spec & 15;
        let packet_type = data.get_u8();
        let cts_unsigned = data.get_u32() & 0x00FFFFFF;
        let cts = (cts_unsigned << 8) >> 8;
    
        // 非关键帧
        // 拆分视频轨道
        if packet_type == 1 {
            return self.parseAVCVideoData(data, timestamp, cts);
        }

        // 关键帧
        // 解析视频编码配置信息
        self.parseAVCDecoderConfigurationRecord(data)
    }

    /// 解析视频关键帧
    /// 
    /// 解析视频编码配置信息
    #[allow(dead_code)]
    fn parseAVCDecoderConfigurationRecord(&mut self, mut data: BytesMut) -> DecoderResult {
        let version = data.get_u8();
        let avc_profile = data.get_u8();
        let profile_compatibility = data.get_u8();
        let avclevel = data.get_u8();
        let nalu_length_size = (data.get_u8() & 3) + 1;
        let sps_count = data.get_u8() & 31;

        self.nalu_length_size = nalu_length_size as usize;
    }

    /// 解析视频帧数据
    /// 
    /// 非关键帧数据
    /// 拆分出视频轨道数据
    #[allow(dead_code)]
    fn parseAVCVideoData(&self, mut data: BytesMut, timestamp: u32, cts: u32) -> DecoderResult {
        let data_size = data.len();
        let mut units = Vec::new();
        let mut offset = 0;
        while offset < data_size {

            // 无法完成下次解码
            // 跳出循环
            if offset + 4 >= data_size {
                break;
            }

            // Nalu with length-header (AVC1)
            let mut naluSize = data.get_u32() as usize;
            if self.nalu_length_size == 3 {
                naluSize >>= 8;
            }

            // 检查是否解析完成
            if naluSize > data_size - self.nalu_length_size {
                break;
            }

            // NAL包类型
            let unitType = data.get_u8() & 0x1F;
            units.push((unitType as usize, data));
            offset += self.nalu_length_size + naluSize;
        }

        // 返回视频单元和控制信息
        DecoderResult::VideoTrack(VideoSample {
            dts: timestamp,
            pts: timestamp + cts,
            units,
            cts,
        })
    }
}
