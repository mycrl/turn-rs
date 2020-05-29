use super::sps::sps_parse;
use super::{Metadata, FrameRate, Tag, DecoerResult, VideoSample};
use bytes::{BytesMut, Buf};

/// 解析视频帧
/// 
/// 注意: 只支持H264
#[allow(dead_code)]
pub fn decoder(mut data: BytesMut, naluLengthSize: usize, timestamp: u32) -> DecoerResult {
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
        return parse_avc_data(data, naluLengthSize, timestamp, cts);
    }

    // 关键帧
    // 获取关键帧数据
    let version = data.get_u8();
    let avc_profile = data.get_u8();
    let profile_compatibility = data.get_u8();
    let avclevel = data.get_u8();
    let nalu_length_size = (data.get_u8() & 3) + 1;
    let sps_count = data.get_u8() & 31;
    
    let mut meta = Metadata::default();
    let ref_sample_duration = 0;

    meta.tag = Tag::Video;
    meta.track_id = 1;
    meta.timescale = 0;
    meta.duration = 1000;

    for i in 0..sps_count {
        let len = data.get_u16();
        if len == 0 {
            continue;
        }

        if i != 0 {
            break;
        }

        let mut config = sps_parse(&data[0..len as usize]);
        meta.codec_width = config.codec_size.width;
        meta.codec_height = config.codec_size.height;
        meta.present_width = config.present_size.width;
        meta.present_height = config.present_size.height;
        meta.profile = config.profile_string;
        meta.level = config.level_string;
        meta.bit_depth = config.bit_depth;
        meta.chroma_format = config.chroma_format;
        meta.sar_ratio = config.sar_ratio;
        meta.frame_rate = config.frame_rate;

        if meta.frame_rate.fixed == false || 
            meta.frame_rate.fps_num == 0 || 
            meta.frame_rate.fps_den == 0 
        {
            config.frame_rate = FrameRate {
                fixed: true,
                fps: 23.976,
                fps_num: 23976,
                fps_den: 1000
            };
        }

        let fps_den = meta.frame_rate.fps_den;
        let fps_num = meta.frame_rate.fps_num;
        meta.ref_sample_duration = meta.timescale * (fps_den / fps_num) as u32;

        let codec_array = &data[1..4];
        let mut codec_string = "avc1.".to_string();
        for i in 0..3 {
            let mut hex = format!("{:x}", codec_array[i]);
            if hex.len() < 2 {
                hex.insert_str(0, "0");
            }

            codec_string.push_str(&hex);
        }

        meta.codec = codec_string;
    }

    data.advance(1);
    meta.avcc = avcc.freeze();
    meta
}
