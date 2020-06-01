use super::{DecoderResult, Tag, Metadata};
use bytes::{BytesMut, Bytes, Buf};

/// 解析音频帧
/// 
/// 注意: 只支持AAC.
/// 输入音频帧获得音频帧信息.
#[rustfmt::skip]
pub fn decoder(mut data: BytesMut, timestamp: u32) -> DecoderResult {

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
    DecoderResult::AudioMetadata(meta)
}
