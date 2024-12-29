use crc::{Crc, CRC_32_ISO_HDLC};
use hmac::{digest::CtOutput, Hmac, Mac};
use md5::{Digest, Md5};

use crate::StunError;

/// compute padding size.
///
/// RFC5766 stipulates that the attribute
/// content is a multiple of 4.
///
/// # Test
///
/// ```
/// assert_eq!(stun::util::pad_size(4), 0);
/// assert_eq!(stun::util::pad_size(0), 0);
/// assert_eq!(stun::util::pad_size(5), 3);
/// ```
#[inline(always)]
pub fn pad_size(size: usize) -> usize {
    let range = size % 4;
    if size == 0 || range == 0 {
        return 0;
    }

    4 - range
}

/// create long term credential.
///
/// > key = MD5(username ":" OpaqueString(realm) ":" OpaqueString(password))
///
/// ```
/// let buffer = [
///     0x3eu8, 0x2f, 0x79, 0x1e, 0x1f, 0x14, 0xd1, 0x73, 0xfc, 0x91, 0xff,
///     0x2f, 0x59, 0xb5, 0x0f, 0xd1,
/// ];
///
/// let key =
///     stun::util::long_term_credential_digest("panda", "panda", "raspberry");
/// assert_eq!(key, buffer);
/// ```
pub fn long_term_credential_digest(username: &str, password: &str, realm: &str) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update([username, realm, password].join(":"));
    hasher.finalize().into()
}

/// HMAC SHA1 digest.
///
/// # Test
///
/// ```
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
/// let hmac_output = stun::util::hmac_sha1(&key, &[&buffer])
///     .unwrap()
///     .into_bytes();
/// assert_eq!(hmac_output.as_slice(), &sign);
/// ```
pub fn hmac_sha1(key: &[u8], source: &[&[u8]]) -> Result<CtOutput<Hmac<sha1::Sha1>>, StunError> {
    match Hmac::<sha1::Sha1>::new_from_slice(key) {
        Err(_) => Err(StunError::SummaryFailed),
        Ok(mut mac) => {
            for buf in source {
                mac.update(buf);
            }

            Ok(mac.finalize())
        }
    }
}

/// CRC32 Fingerprint.
///
/// # Test
///
/// ```
/// assert_eq!(stun::util::fingerprint(b"1"), 3498621689);
/// ```
pub fn fingerprint(bytes: &[u8]) -> u32 {
    Crc::<u32>::new(&CRC_32_ISO_HDLC).checksum(bytes) ^ 0x5354_554e
}
