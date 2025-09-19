use aws_lc_rs::{digest, hmac};
use md5::{Digest, Md5}; // aws-lc-rs不支持MD5，保留

use crate::Error;

/// HMAC SHA1 digest.
///
/// # Test
///
/// ```
/// use turn_server_codec::crypto::hmac_sha1;
///
/// let buffer = [
///     0x00u8, 0x03, 0x00, 0x50, 0x21, 0x12, 0xa4, 0x42, 0x64, 0x4f, 0x5a,
///     0x78, 0x6a, 0x56, 0x33, 0x62, 0x4b, 0x52, 0x33, 0x31, 0x00, 0x19, 0x00,
///     0x04, 0x11, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x05, 0x70, 0x61, 0x6e,
///     0x64, 0x61, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x09, 0x72, 0x61, 0x73,
///     0x70, 0x62, 0x65, 0x72, 0x72, 0x79, 0x00, 0x00, 0x00, 0x00, 0x15, 0x00,
///     0x10, 0x31, 0x63, 0x31, 0x33, 0x64, 0x32, 0x62, 0x32, 0x34, 0x35, 0x62,
///     0x33, 0x61, 0x37, 0x33, 0x34,
/// ];
///
/// let key = [
///     0x3eu8, 0x2f, 0x79, 0x1e, 0x1f, 0x14, 0xd1, 0x73, 0xfc, 0x91, 0xff,
///     0x2f, 0x59, 0xb5, 0x0f, 0xd1,
/// ];
///
/// let sign = [
///     0xd6u8, 0x78, 0x26, 0x99, 0x0e, 0x15, 0x56, 0x15, 0xe5, 0xf4, 0x24,
///     0x74, 0xe2, 0x3c, 0x26, 0xc5, 0xb1, 0x03, 0xb2, 0x6d,
/// ];
///
/// let hmac_output = hmac_sha1(&key, &[&buffer])
///     .unwrap();
///
/// assert_eq!(&hmac_output, &sign);
/// ```
pub fn hmac_sha1(key: &[u8], source: &[&[u8]]) -> Result<[u8; 20], Error> {
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, key);
    let mut ctx = hmac::Context::with_key(&key);

    for buf in source {
        ctx.update(buf);
    }

    let signature = ctx.sign();
    let mut result = [0u8; 20];
    result.copy_from_slice(signature.as_ref());
    Ok(result)
}

/// CRC32 Fingerprint.
///
/// # Test
///
/// ```
/// use turn_server_codec::crypto::fingerprint;
///
/// assert_eq!(fingerprint(b"1"), 3498621689);
/// ```
pub fn fingerprint(bytes: &[u8]) -> u32 {
    crc32fast::hash(bytes) ^ 0x5354_554e
}

/// create long term credential for md5.
///
/// > key = MD5(username ":" OpaqueString(realm) ":" OpaqueString(password))
///
/// # Test
///
/// ```
/// use turn_server_codec::crypto::password_md5;
///
/// let buffer = [
///     0x3eu8, 0x2f, 0x79, 0x1e, 0x1f, 0x14, 0xd1, 0x73, 0xfc, 0x91, 0xff,
///     0x2f, 0x59, 0xb5, 0x0f, 0xd1,
/// ];
///
/// let key = password_md5(
///     "panda",
///     "panda",
///     "raspberry",
/// );
///
/// assert_eq!(key, buffer);
/// ```
pub fn password_md5(username: &str, password: &str, realm: &str) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update([username, realm, password].join(":"));
    hasher.finalize().into()
}

/// create long term credential for sha256.
///
/// > key = SHA256(username ":" OpaqueString(realm) ":" OpaqueString(password))
///
/// # Test
///
/// ```
/// use turn_server_codec::crypto::password_sha256;
///
/// let key = password_sha256(
///     "panda",
///     "panda",
///     "raspberry",
/// );
///
/// // SHA256 produces 32 bytes
/// assert_eq!(key.len(), 32);
/// ```
pub fn password_sha256(username: &str, password: &str, realm: &str) -> [u8; 32] {
    let mut ctx = digest::Context::new(&digest::SHA256);
    let input = [username, realm, password].join(":");
    ctx.update(input.as_bytes());
    let digest = ctx.finish();
    let mut result = [0u8; 32];
    result.copy_from_slice(digest.as_ref());
    result
}
