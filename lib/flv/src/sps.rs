use super::{SPS, FrameRate, Size};
use bytes::{BytesMut, BufMut, Bytes};
use golomb::ExpGolomb;

const SAR_WIDTH_TABLE: [u8; 16] = [1, 12, 10, 16, 40, 24, 20, 32, 80, 18, 15, 64, 160, 4, 3, 2];
const SAR_HEIGHT_TABLE: [u8; 16] = [1, 11, 11, 11, 33, 11, 11, 11, 33, 11, 11, 33,  99, 3, 2, 1];
const PROFILE_IDCS: [u8; 11] = [ 44, 83, 86, 100, 110, 122, 244, 118, 128, 138, 144];
const CHROMA_FORMAT_TABLE: [u32; 4] = [0, 420, 422, 444];

pub fn ebsp_rbsp(data: &[u8]) -> Bytes {
    let mut buffer = BytesMut::new();
    let mut index = 0;
    for i in 0..data.len() {
        if i >= 2  {
            if data[i] == 0x03 {
                if data[i - 1] == 0x00 {
                    if data[i - 2] == 0x00 {
                        continue;
                    }
                }
            }
        }

        buffer.put_u8(data[i]);
        index += 1;
    }

    buffer
        .split_to(index)
        .freeze()
}

fn get_profile_string(profile_idc: u8) -> String {
    match profile_idc {
        66 => "Baseline",
        77 => "Main",
        88 => "Extended",
        100 => "High",
        110 => "High10",
        122 => "High422",
        244 => "High444",
        _ => "Unknown"
    }.to_string()
}

fn get_level_string(level_idc: u8) -> String {
    (level_idc / 10).to_string()
}

fn skip_scaling_list(gb: &mut ExpGolomb, count: usize) {
    let mut last_scale = 8;
    let mut next_scale = 8;
    for _ in 0..count {
        if next_scale != 0 {
            let delta_scale = gb.read_seg();
            next_scale = (last_scale + delta_scale + 256) % 256;
        }

        last_scale = if next_scale == 0 { 
            last_scale 
         } else {
            next_scale
         };
    }
}

fn get_chroma_format_string(chroma: u32) -> String {
    match chroma {
        420 => "4:2:0",
        422 => "4:2:2",
        444 => "4:4:4",
        _ => "Unknown"
    }.to_string()
}

#[allow(unused_assignments)]
pub fn sps_parse(data: &[u8]) -> SPS {
    let ebsp = ebsp_rbsp(&data);
    let mut golomb = ExpGolomb::new(&ebsp[..]);

    golomb.read_byte();
    let profile_idc = golomb.read_byte();
    golomb.read_byte();
    let level_idc = golomb.read_byte();
    golomb.read_ueg();

    let profile_string = get_profile_string(profile_idc);
    let level_string = get_level_string(level_idc);
    let mut chroma_format_idc = 1;
    let mut chroma_format = 420;
    let mut bit_depth = 8;

    if PROFILE_IDCS.contains(&profile_idc) {
        chroma_format_idc = golomb.read_ueg();
        if chroma_format_idc == 3 {
            golomb.read_bits(1);
        }

        if chroma_format_idc <= 3 {
            chroma_format = CHROMA_FORMAT_TABLE[chroma_format_idc as usize];
        }

        bit_depth = golomb.read_ueg() + 8;
        golomb.read_ueg();
        golomb.read_bits(1);

        if golomb.read_bool() {
            let scaling_list_count = if chroma_format_idc != 3 { 8 } else { 12 };
            for i in 0..scaling_list_count {
                if golomb.read_bool() {
                    if i < 6 {
                        skip_scaling_list(&mut golomb, 16);
                    } else {
                        skip_scaling_list(&mut golomb, 64);
                    }
                }
            }
        }
    }

    golomb.read_ueg();
    let pic_order_cnt_type = golomb.read_ueg();
    if pic_order_cnt_type == 0 {
        golomb.read_ueg();
    } else
    if pic_order_cnt_type == 1 {
        golomb.read_bits(1);
        golomb.read_seg();
        golomb.read_seg();
        for _ in 0..golomb.read_ueg() {
            golomb.read_seg();
        }
    }

    let ref_frames = golomb.read_ueg();
    golomb.read_bits(1);

    let pic_width_in_mbs_minus1 = golomb.read_ueg();
    let pic_height_in_map_units_minus1= golomb.read_ueg();
    let frame_mbs_only_flag = golomb.read_bits(1);
    if frame_mbs_only_flag == 0 {
        golomb.read_bits(1);
    }

    golomb.read_bits(1);
    let mut frame_crop_left_offset = 0;
    let mut frame_crop_right_offset = 0;
    let mut frame_crop_top_offset = 0;
    let mut frame_crop_bottom_offset = 0;
    if golomb.read_bool() {
        frame_crop_left_offset = golomb.read_ueg();
        frame_crop_right_offset = golomb.read_ueg();
        frame_crop_top_offset = golomb.read_ueg();
        frame_crop_bottom_offset = golomb.read_ueg();
    }

    let mut sar_width = 1;
    let mut sar_height = 1;
    let mut fps = 0.0;
    let mut fps_fixed = true;
    let mut fps_num = 0;
    let mut fps_den = 0;
    if golomb.read_bool() {
        if golomb.read_bool() {
            let aspect_ratio_idc = golomb.read_byte();
            if aspect_ratio_idc > 0 && aspect_ratio_idc < 16 {
                sar_width = SAR_WIDTH_TABLE[(aspect_ratio_idc - 1) as usize];
                sar_height = SAR_HEIGHT_TABLE[(aspect_ratio_idc - 1) as usize];
            } else
            if aspect_ratio_idc == 255 {
                sar_width = ((golomb.read_byte() as usize) << 8 | golomb.read_byte() as usize) as u8;
                sar_height = ((golomb.read_byte() as usize) << 8 | golomb.read_byte() as usize) as u8;
            }
        }

        if golomb.read_bool() {
            golomb.read_bool();
        }

        if golomb.read_bool() {
            golomb.read_bits(4);
            if golomb.read_bool() {
                golomb.read_bits(24);
            }
        }

        if golomb.read_bool() {
            golomb.read_ueg();
            golomb.read_ueg();
        }

        if golomb.read_bool() {
            let num_units_in_tick = golomb.read_bits(32);
            let time_scale = golomb.read_bits(32);
            fps_fixed = golomb.read_bool();
            fps_num = time_scale;
            fps_den = num_units_in_tick * 2;
            fps = fps_num as f64 / fps_den as f64;
        }
    }

    let mut sar_scale = 1;
    if sar_width != 1 || sar_height != 1 {
        sar_scale = sar_width / sar_height;
    }

    let mut crop_unit_x = 0;
    let mut crop_unit_y = 0;
    if chroma_format_idc == 0 {
        crop_unit_x = 1;
        crop_unit_y = 2 - frame_mbs_only_flag;
    } else {
        let sub_wc = if chroma_format_idc == 3 { 1 } else { 2 };
        let sub_hc = if chroma_format_idc == 1 { 2 } else { 1 };
        crop_unit_y = sub_hc * (2 - frame_mbs_only_flag);
        crop_unit_x = sub_wc;
    }

    let mut codec_width = (pic_width_in_mbs_minus1 + 1) * 16;
    let mut codec_height = (2 - frame_mbs_only_flag) * ((pic_height_in_map_units_minus1 + 1) * 16);
    codec_width -= (frame_crop_left_offset + frame_crop_right_offset) * crop_unit_x;
    codec_height -= (frame_crop_top_offset + frame_crop_bottom_offset) * crop_unit_y;
    let present_width = codec_width * sar_scale as u32;

    SPS {
        profile_string,
        level_string,
        bit_depth,
        ref_frames,
        chroma_format,
        chroma_format_string: get_chroma_format_string(chroma_format),
        frame_rate: FrameRate {
            fixed: fps_fixed,
            fps_den: fps_den,
            fps_num: fps_num,
            fps: fps
        },
        sar_ratio: Size {
            width: sar_width as u32,
            height: sar_height as u32
        },
        codec_size: Size {
            width: codec_width,
            height: codec_height
        },
        present_size: Size {
            width: present_width,
            height: codec_height
        }
    }
}
