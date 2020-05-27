use bytes::{BytesMut, BufMut, Buf};
use super::exp_golomb::ExpGolomb;

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

pub fn parse(mut data: BytesMut) {
    let rbsp = ebsp2rbsp(&data);
    let mut gb = ExpGolomb::new(&rbsp);

    gb.read_byte();
    let profile_idc = gb.read_byte();  // profile_idc
    gb.read_byte();  // constraint_set_flags[5] + reserved_zero[3]
    let level_idc = gb.read_byte();  // level_idc
    gb.read_ueg();  // seq_parameter_set_id

    let profile_string = get_profile_string(profile_idc);
}

pub fn get_profile_string(profile_idc: usize) -> &'static str {
    match profile_idc {
        66 => "Baseline",
        77 => "Main",
        88 => "Extended",
        100 => "High",
        110 => "High10",
        122 => "High422",
        244 => "High444",
        _ => "Unknown"
    }
}
