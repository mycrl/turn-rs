use bytes::{Bytes, BytesMut, Buf};
use super::Metadata;

/// 解析视频帧
/// 
/// 注意: 只支持H264
pub fn decoder(mut data: BytesMut) {
    let video_spec = data.get_u8();
    let video_frame = (video_spec & 240) >> 4;
    let codec_id = video_spec & 15;
    let packet_type = data.get_u8();
    let cts_unsigned = data.get_u32() & 0x00FFFFFF;
    let cts = (cts_unsigned << 8) >> 8;
    let version = data.get_u8();
    let avc_profile = data.get_u8();
    let profile_compatibility = data.get_u8();
    let avclevel = data.get_u8();
    let nalu_length_size = (data.get_u8() & 3) + 1;
    let sps_count = data.get_u8() & 31;

    for _ in 0..sps_count {
        let len = data.get_u16();
        if len == 0 {
            continue;
        }

        
    }
}
