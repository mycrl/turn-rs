use rand::Rng;


/// # Generate rand string.
/// 
pub fn rand_strs (length: u64) -> Vec<u8> {
    let mut strs = Vec::new();
    let mut index = 0;

    // loop count.
    while index < length {
        strs.push(rand::thread_rng().gen_range(0, 255));
        index += 1;
    }

    strs
}