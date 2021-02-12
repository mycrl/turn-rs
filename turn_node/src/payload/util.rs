use anyhow::{anyhow, Result};
use crc::crc32;
use hmac::crypto_mac::Output;
use hmac::{Hmac, Mac, NewMac};

/// 计算填充位
///
/// RFC5766规定属性内容是4的倍数，
/// 所以此处是为了计算出填充位的长度.
#[inline(always)]
pub fn pad_size(size: usize) -> usize {
    let range = size % 4;
    if size == 0 || range == 0 {
        return 0;
    }

    4 - range
}

/// 计算长期凭证
///
/// > key = MD5(username ":" OpaqueString(realm) ":" OpaqueString(password))
pub fn long_key(username: &str, key: &str, realm: &str) -> [u8; 16] {
    md5::compute([username, realm, key].join(":")).0
}

/// HMAC SHA1 摘要
pub fn hmac_sha1(key: &[u8], source: Vec<&[u8]>) -> Result<Output<Hmac<sha1::Sha1>>> {
    match Hmac::<sha1::Sha1>::new_varkey(key) {
        Err(_) => Err(anyhow!("new key failde")),
        Ok(mut mac) => {
            for buf in source {
                mac.update(buf);
            }

            Ok(mac.finalize())
        }
    }
}

/// CRC32 Fingerprint
pub fn fingerprint(buffer: &[u8]) -> u32 {
    crc32::checksum_ieee(buffer) ^ 0x5354_554e
}

/// 方便得将缓冲区转为U16
#[inline(always)]
pub fn as_u16(buf: &[u8]) -> u16 {
    assert!(buf.len() >= 2);
    u16::from_be_bytes([buf[0], buf[1]])
}

/// 方便得将缓冲区转为U32
#[inline(always)]
pub fn as_u32(buf: &[u8]) -> u32 {
    assert!(buf.len() >= 4);
    u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]])
}
