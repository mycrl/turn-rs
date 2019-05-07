use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};


/// # Generate rand string.
/// 
pub fn rand_numbers (length: u64) -> Vec<u8> {
    let mut strs = Vec::new();
    let mut index = 0;

    // loop count.
    while index < length {
        strs.push(rand::thread_rng().gen_range(0, 255));
        index += 1;
    }

    strs
}


/// # Get system timestamp.
/// 
pub fn timestamp () -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    let second = since_the_epoch.as_secs() as i64 * 1000i64 + (since_the_epoch.subsec_nanos() as f64 / 1_000_000.0) as i64;
    second as u64
}