use super::exp_golomb::ExpGolomb;
use super::{SPS, FrameRate, Size};
use bytes::{BytesMut, BufMut};

pub fn ebsp2rbsp(data: &[u8]) -> BytesMut {
    let mut dst = BytesMut::new();
    for i in 0..data.len() {
        if i >= 2 {
            let bit_1 = data[i] == 0x03;
            let bit_2 = data[i - 1] == 0x00;
            let bit_3 = data[i - 2] == 0x00;
            if bit_1 && bit_2 && bit_3 {
                continue;
            }
        }

        dst.put_u8(data[i]);
    }

    dst
}

pub fn sps_parse(data: &[u8]) -> SPS {
    let rbsp = ebsp2rbsp(&data);
    let mut gb = ExpGolomb::new(&rbsp);

    gb.read_byte();
    let profile_idc = gb.read_byte();  // profile_idc
    gb.read_byte();  // constraint_set_flags[5] + reserved_zero[3]
    let level_idc = gb.read_byte();  // level_idc
    gb.read_ueg();  // seq_parameter_set_id

    let profile_string = get_profile_string(profile_idc);
    let level_string = get_level_string(level_idc);
    let mut chroma_format_idc = 1;
    let mut chroma_format = 420;
    let chroma_format_table = [0, 420, 422, 444];
    let mut bit_depth = 8;

    if profile_idc == 100 || profile_idc == 110 || profile_idc == 122 ||
        profile_idc == 244 || profile_idc == 44 || profile_idc == 83 ||
        profile_idc == 86 || profile_idc == 118 || profile_idc == 128 ||
        profile_idc == 138 || profile_idc == 144 {
        
        chroma_format_idc = gb.read_ueg();
        if chroma_format_idc == 3 {
            gb.read_bits(1);
        }

        if chroma_format_idc <= 3 {
            chroma_format = chroma_format_table[chroma_format_idc];
        }

        bit_depth = gb.read_ueg() + 8;  // bit_depth_luma_minus8
        gb.read_ueg();
        gb.read_bits(1);
        if gb.read_bool() {
            let scaling_list_count = if chroma_format_idc != 3 {  8 } else { 12 };
            for i in 0..scaling_list_count {
                if gb.read_bool() {
                    skip_scaling_list(&mut gb, if i < 6 { 16 } else { 64 });
                }
            }
        }
    }

    gb.read_ueg();
    let pic_order_cnt_type = gb.read_ueg();
    if pic_order_cnt_type == 0 {
        gb.read_ueg();
    } else
    if pic_order_cnt_type == 1 {
        gb.read_bits(1);
        gb.read_seg();
        gb.read_seg();
        for _ in 0..gb.read_ueg() {
            gb.read_seg();
        }
    }

    let ref_frames = gb.read_ueg();
    gb.read_bits(1);

    let pic_width_in_mbs_minus1 = gb.read_ueg();
    let pic_height_in_map_units_minus1 = gb.read_ueg();
    let frame_mbs_only_flag = gb.read_bits(1);
    if frame_mbs_only_flag == 0 {
        gb.read_bits(1);
    }

    gb.read_bits(1);
    let mut frame_crop_left_offset = 0;
    let mut frame_crop_right_offset = 0;
    let mut frame_crop_top_offset = 0;
    let mut frame_crop_bottom_offset = 0;
    let frame_cropping_fla = gb.read_bool();
    if frame_cropping_fla {
        frame_crop_left_offset = gb.read_ueg();
        frame_crop_right_offset = gb.read_ueg();
        frame_crop_top_offset = gb.read_ueg();
        frame_crop_bottom_offset = gb.read_ueg();
    }

    let mut sar_width = 1; 
    let mut sar_height = 1;
    let mut fps = 0.0; 
    let mut fps_fixed = true; 
    let mut fps_num = 0; 
    let mut fps_den = 0;
    let vui_parameters_present_flag = gb.read_bool();
    if vui_parameters_present_flag {
        if gb.read_bool() {
            let aspect_ratio_idc = gb.read_byte();
            let sar_w_table = [1, 12, 10, 16, 40, 24, 20, 32, 80, 18, 15, 64, 160, 4, 3, 2];
            let sar_h_table = [1, 11, 11, 11, 33, 11, 11, 11, 33, 11, 11, 33,  99, 3, 2, 1];
            if aspect_ratio_idc > 0 && aspect_ratio_idc < 16 {
                sar_width = sar_w_table[aspect_ratio_idc - 1];
                sar_height = sar_h_table[aspect_ratio_idc - 1];
            } else 
            if aspect_ratio_idc == 255 {
                sar_width = gb.read_byte() << 8 | gb.read_byte();
                sar_height = gb.read_byte() << 8 | gb.read_byte();
            }
        }

        if gb.read_bool() {  // overscan_info_present_flag
            gb.read_bool();  // overscan_appropriate_flag
        }

        if gb.read_bool() {  // video_signal_type_present_flag
            gb.read_bits(4);  // video_format & video_full_range_flag
            if gb.read_bool() {  // colour_description_present_flag
                gb.read_bits(24);  // colour_primaries & transfer_characteristics & matrix_coefficients
            }
        }

        if gb.read_bool() {  // chroma_loc_info_present_flag
            gb.read_ueg();  // chroma_sample_loc_type_top_field
            gb.read_ueg();  // chroma_sample_loc_type_bottom_field
        }

        if gb.read_bool() {  // timing_info_present_flag
            let num_units_in_tick = gb.read_bits(32);
            let time_scale = gb.read_bits(32);
            fps_fixed = gb.read_bool();  // fixed_frame_rate_flag

            fps_num = time_scale;
            fps_den = num_units_in_tick * 2;
            fps = (fps_num / fps_den) as f64;
        }
    }

    let mut sarScale = 1;
    if sar_width != 1 || sar_height != 1 {
        sarScale = sar_width / sar_height;
    }

    let mut crop_unit_x = 0; 
    let mut crop_unit_y = 0;
    if chroma_format_idc == 0 {
        crop_unit_x = 1;
        crop_unit_y = 2 - frame_mbs_only_flag;
    } else {
        let sub_wc = if chroma_format_idc == 3 {  1 } else { 2 };
        let sub_hc = if chroma_format_idc == 1 { 2 } else { 1 };
        crop_unit_x = sub_wc;
        crop_unit_y = sub_hc * (2 - frame_mbs_only_flag);
    }

    let mut codec_width = (pic_width_in_mbs_minus1 + 1) * 16;
    let mut codec_height = (2 - frame_mbs_only_flag) * ((pic_height_in_map_units_minus1 + 1) * 16);
    codec_width -= (frame_crop_left_offset + frame_crop_right_offset) * crop_unit_x;
    codec_height -= (frame_crop_top_offset + frame_crop_bottom_offset) * crop_unit_y;
    let present_width = ((codec_width * sarScale) as f64).ceil() as usize;

    SPS {
        profile_string,
        level_string,
        bit_depth,
        ref_frames,
        chroma_format,
        chroma_format_string: get_chroma_format_string(chroma_format),
        frame_rate: FrameRate {
            fps,
            fps_den,
            fps_num,
            fixed: fps_fixed,
        },
        sar_ratio: Size {
            width: sar_width,
            height: sar_height
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

fn get_profile_string(profile_idc: usize) -> String {
    (match profile_idc {
        66 => "Baseline",
        77 => "Main",
        88 => "Extended",
        100 => "High",
        110 => "High10",
        122 => "High422",
        244 => "High444",
        _ => "Unknown"
    }).to_string()
}

fn get_level_string(level_idc: usize) -> String {
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

fn get_chroma_format_string(chroma: usize) -> String {
    (match chroma {
        420 => "4:2:0",
        422 => "4:2:2",
        444 => "4:4:4",
        _ => "Unknown"
    }).to_string()
}
