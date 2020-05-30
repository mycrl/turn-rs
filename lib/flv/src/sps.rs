use super::{SPS, FrameRate, Size};
use bytes::{BytesMut, BufMut};
use golomb::ExpGolomb;

const CHROMA_FORMAT_TABLE: [u8; 4] = [0, 420, 422, 444];
const PROFILE_IDCS: [u8; 11] = [
    44, 83, 86, 100, 110, 122, 
    244, 118, 128, 138, 144
];

pub fn ebsp_rbsp(data: &[u8]) -> &[u8] {
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

    &buffer[0..index]
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

fn get_chroma_format_string(chroma: usize) -> String {
    match chroma {
        420 => "4:2:0",
        422 => "4:2:2",
        444 => "4:4:4",
        _ => "Unknown"
    }.to_string()
}

pub fn sps_parse(data: &[u8]) {
    let mut golomb = ExpGolomb::new(ebsp_rbsp(&data));

    golomb.read_byte();
    let profile_idc = golomb.read_byte();
    golomb.read_byte();
    let level_idc = golomb.read_byte();
    golomb.read_ueg();

    let profile_string = get_profile_string(profile_idc);
    let level_string = get_level_string(level_idc);
    let chroma_format_idc = 1;
    let chroma_format = 420;
    let bit_depth = 8;

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
        let pic_height_in_map_unints_minus1 = golomb.read_ueg();
        let frame_mbs_only_flag = golomb.read_bits(1);
        if frame_mbs_only_flag == 0 {
            golomb.read_bits(1);
        }

        golomb.read_bits(1);
        let frame_crop_left_offset = 0;
    }
}
