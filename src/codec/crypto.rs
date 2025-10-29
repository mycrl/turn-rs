use std::ops::Deref;

use aws_lc_rs::{digest, hmac};
use base64::{Engine, prelude::BASE64_STANDARD};
use md5::{Digest, Md5};

use super::message::attributes::PasswordAlgorithm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Password {
    Md5([u8; 16]),
    Sha256([u8; 32]),
}

impl Deref for Password {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            Password::Md5(it) => it,
            Password::Sha256(it) => it,
        }
    }
}

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
/// let hmac_output = hmac_sha1(&key, &[&buffer]);
///
/// assert_eq!(&hmac_output, &sign);
/// ```
pub fn hmac_sha1(key: &[u8], source: &[&[u8]]) -> [u8; 20] {
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, key);
    let mut ctx = hmac::Context::with_key(&key);

    for buf in source {
        ctx.update(buf);
    }

    let mut result = [0u8; 20];
    result.copy_from_slice(ctx.sign().as_ref());
    result
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

/// generate create long term credential.
///
/// > key = MD5(username ":" OpaqueString(realm) ":" OpaqueString(password))
///
/// # Test
///
/// ```
/// use turn_server_codec::crypto::{generate_password, Password};
/// use turn_server_codec::message::attributes::PasswordAlgorithm;
///
/// let buffer = [
///     0x3eu8, 0x2f, 0x79, 0x1e, 0x1f, 0x14, 0xd1, 0x73, 0xfc, 0x91, 0xff,
///     0x2f, 0x59, 0xb5, 0x0f, 0xd1,
/// ];
///
/// let password = generate_password(
///     "panda",
///     "panda",
///     "raspberry",
///     PasswordAlgorithm::Md5,
/// );
///
/// match password {
///     Password::Md5(it) => {
///         assert_eq!(it, buffer);
///     }
///     Password::Sha256(it) => {
///         unreachable!();
///     }
/// }
/// ```
pub fn generate_password(
    username: &str,
    password: &str,
    realm: &str,
    algorithm: PasswordAlgorithm,
) -> Password {
    match algorithm {
        PasswordAlgorithm::Md5 => {
            let mut hasher = Md5::new();

            hasher.update([username, realm, password].join(":"));

            Password::Md5(hasher.finalize().into())
        }
        PasswordAlgorithm::Sha256 => {
            let mut ctx = digest::Context::new(&digest::SHA256);

            ctx.update([username, realm, password].join(":").as_bytes());

            let mut result = [0u8; 32];
            result.copy_from_slice(ctx.finish().as_ref());
            Password::Sha256(result)
        }
    }
}

// Because (TURN REST api) this RFC does not mandate the format of the username,
// only suggested values. In principle, the RFC also indicates that the
// timestamp part of username can be set at will, so the timestamp is not
// verified here, and the external web service guarantees its security by
// itself.
//
// https://datatracker.ietf.org/doc/html/draft-uberti-behave-turn-rest-00#section-2.2
pub fn static_auth_secret(
    username: &str,
    secret: &str,
    realm: &str,
    algorithm: PasswordAlgorithm,
) -> Password {
    let password =
        BASE64_STANDARD.encode(hmac_sha1(secret.as_bytes(), &[username.as_bytes()]).as_slice());

    generate_password(username, &password, realm, algorithm)
}
