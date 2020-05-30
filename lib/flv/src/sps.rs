use super::{SPS, FrameRate, Size};
use bytes::{BytesMut, BufMut};
use golomb::ExpGolomb;

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

fn get_profile_string(profile_idc: usize) -> String {
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
    match chroma {
        420 => "4:2:0",
        422 => "4:2:2",
        444 => "4:4:4",
        _ => "Unknown"
    }.to_string()
}

pub fn sps_parse(data: &[u8]) {
    let mut golomb = ExpGolomb::new(ebsp_rbsp(&data));

}
