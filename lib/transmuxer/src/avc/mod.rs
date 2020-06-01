mod sps;

use crate::DecoderResult;
use crate::{Metadata, VideoSample, Tag, FrameRate};
use bytes::{BytesMut, Buf};

/// AVC解码器
pub struct AVC {
    nalu_length_size: usize
}

impl AVC {
    pub fn new() -> Self {
        Self {
            nalu_length_size: 0
        }
    }

    /// 解析视频帧
    /// 
    /// 注意: 只支持H264
    #[rustfmt::skip]
    pub fn decoder(&mut self, data: &mut BytesMut, timestamp: u32) -> DecoderResult {
        data.advance(1);
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
    #[allow(bad_style)]
    fn parseAVCDecoderConfigurationRecord(&mut self, data: &mut BytesMut) -> DecoderResult {
        let avcc = data.clone();
        let mut meta = Metadata::default();
        data.advance(4);

        meta.tag = Tag::Video;
        meta.track_id = 1;
        meta.duration = 0;
        meta.timescale = 1000;
        self.nalu_length_size = ((data.get_u8() & 3) + 1) as usize;

        let sps_count = data.get_u8() & 31;
        for i in 0..sps_count {
            let len = data.get_u16();
            if len == 0 {
                continue;
            }

            let sps = data.split_to(len.into());
            let config = sps::sps_parse(&sps);
            if i != 0 {
                continue;
            }

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

            let fif = meta.frame_rate.fixed == false;
            let fiz = meta.frame_rate.fps_num == 0;
            let fpiz = meta.frame_rate.fps_den == 0;
            if fif || fiz || fpiz {
                meta.frame_rate = FrameRate {
                    fixed: true,
                    fps: 23.976,
                    fps_num: 23976,
                    fps_den: 1000
                };
            }

            let fps_den = meta.frame_rate.fps_den;
            let fps_num = meta.frame_rate.fps_num;
            meta.ref_sample_duration = meta.timescale * (fps_den / fps_num);

            let code_array = &sps[1..4];
            meta.codec = String::from("avc1.");
            for i in 0..3 {
                let mut hex = format!("{:x}", code_array[i]);
                if hex.len() < 2 {
                    hex.insert(0, '0');
                }

                meta.codec.push_str(&hex);
            }
        }

        meta.avcc = avcc.freeze();
        DecoderResult::VideoMetadata(meta)
    }

    /// 解析视频帧数据
    /// 
    /// 非关键帧数据
    /// 拆分出视频轨道数据
    #[allow(bad_style)]
    fn parseAVCVideoData(&self, data: &mut BytesMut, timestamp: u32, cts: u32) -> DecoderResult {
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
            let mut nalu_size = data.get_u32() as usize;
            if self.nalu_length_size == 3 {
                nalu_size >>= 8;
            }

            // 检查是否解析完成
            if nalu_size > data_size - self.nalu_length_size {
                break;
            }

            // NAL包类型
            let unit_type = data.get_u8() & 0x1F;
            units.push((unit_type as usize, data.clone()));
            offset += self.nalu_length_size + nalu_size;
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
