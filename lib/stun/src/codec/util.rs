use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

/// 计算填充位
///
/// RFC5766规定属性内容是4的倍数，
/// 所以此处是为了计算出填充位的长度.
#[rustfmt::skip]
pub fn pad_size(size: usize) -> usize {
    let range = size % 4;
    if size == 0 || range == 0 { return 0; }
    4 - range
}

/// 随机字符串
/// 
/// NONCE
/// The NONCE attribute may be present in requests and responses.  It
/// contains a sequence of qdtext or quoted-pair, which are defined in
/// RFC 3261 [RFC3261].  Note that this means that the NONCE attribute
/// will not contain actual quote characters.  See RFC 2617 [RFC2617],
/// Section 4.3, for guidance on selection of nonce values in a server.
///
/// It MUST be less than 128 characters (which can be as long as 763
/// bytes).
pub fn rand_string(size: usize) -> String {
    let mut rng = thread_rng();
    std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(size)
        .collect()
}

/// 消息完整性
/// 
/// 对关键属性摘要.
/// 
/// The MESSAGE-INTEGRITY attribute contains an HMAC-SHA1 [RFC2104] of
/// the STUN message.  The MESSAGE-INTEGRITY attribute can be present in
/// any STUN message type.  Since it uses the SHA1 hash, the HMAC will be
/// 20 bytes.  The text used as input to HMAC is the STUN message,
/// including the header, up to and including the attribute preceding the
/// MESSAGE-INTEGRITY attribute.  With the exception of the FINGERPRINT
/// attribute, which appears after MESSAGE-INTEGRITY, agents MUST ignore
/// all other attributes that follow MESSAGE-INTEGRITY.
/// 
/// The key for the HMAC depends on whether long-term or short-term
/// credentials are in use.  For long-term credentials, the key is 16
/// bytes:
/// 
///          key = MD5(username ":" realm ":" SASLprep(password))
/// 
/// That is, the 16-byte key is formed by taking the MD5 hash of the
/// result of concatenating the following five fields: (1) the username,
/// with any quotes and trailing nulls removed, as taken from the
/// USERNAME attribute (in which case SASLprep has already been applied);
/// (2) a single colon; (3) the realm, with any quotes and trailing nulls
/// removed; (4) a single colon; and (5) the password, with any trailing
/// nulls removed and after processing using SASLprep.  For example, if
/// the username was 'user', the realm was 'realm', and the password was
/// 'pass', then the 16-byte HMAC key would be the result of performing
/// an MD5 hash on the string 'user:realm:pass', the resulting hash being
/// 0x8493fbc53ba582fb4c044c456bdc40eb.
/// 
/// For short-term credentials:
/// 
///                        key = SASLprep(password)
/// 
/// where MD5 is defined in RFC 1321 [RFC1321] and SASLprep() is defined
/// in RFC 4013 [RFC4013].
/// 
/// The structure of the key when used with long-term credentials
/// facilitates deployment in systems that also utilize SIP.  Typically,
/// SIP systems utilizing SIP's digest authentication mechanism do not
/// actually store the password in the database.  Rather, they store a
/// value called H(A1), which is equal to the key defined above.
/// 
/// Based on the rules above, the hash used to construct MESSAGE-
/// INTEGRITY includes the length field from the STUN message header.
/// Prior to performing the hash, the MESSAGE-INTEGRITY attribute MUST be
/// inserted into the message (with dummy content).  The length MUST then
/// be set to point to the length of the message up to, and including,
/// the MESSAGE-INTEGRITY attribute itself, but excluding any attributes
/// after it.  Once the computation is performed, the value of the
/// MESSAGE-INTEGRITY attribute can be filled in, and the value of the
/// length in the STUN header can be set to its correct value -- the
/// length of the entire message.  Similarly, when validating the
/// MESSAGE-INTEGRITY, the length field should be adjusted to point to
/// the end of the MESSAGE-INTEGRITY attribute prior to calculating the
/// HMAC.  Such adjustment is necessary when attributes, such as
/// FINGERPRINT, appear after MESSAGE-INTEGRITY.
pub fn key_sign(username: String, realm: String, key: String) -> String {
    format!("{:x}", md5::compute([username, realm, key].join(":")))
}
