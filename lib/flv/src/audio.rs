use bytes::{Bytes, BytesMut};
use super::Metadata;

/// 解析音频帧
/// 
/// 注意: 只支持AAC.
/// 输入音频帧获得音频帧信息.
pub fn decoder(data: BytesMut) -> Metadata {
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

    Metadata {
        track_id: 2,
        duration: 0,
        timescale: 1000,
        audio_sample_rate: sampling_frequence,
        channel_count: channel_config,
        codec: "mp4a.40.5".to_string(),
        original_codec: "mp4a.40.5".to_string(),
        config: Bytes::from(config.to_vec()),
        ref_sample_duration: 1024 / sampling_frequence * 1000
    }
}
