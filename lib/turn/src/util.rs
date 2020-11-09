use rand::{
    distributions::Alphanumeric, 
    thread_rng, 
    Rng
};

/// 计算填充位
///
/// RFC5766规定属性内容是4的倍数，
/// 所以此处是为了计算出填充位的长度.
pub fn pad_size(size: usize) -> usize {
    let range = size % 4;
    if size == 0 || range == 0 {
        return 0;
    }
    
    4 - range
}

/// 随机字符串
pub fn rand_string(size: usize) -> String {
    let mut rng = thread_rng();
    std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(size)
        .collect()
}

/// 消息完整性
pub fn key_sign(username: String, realm: String, key: String) -> String {
    format!("{:x}", md5::compute([username, realm, key].join(":")))
}
