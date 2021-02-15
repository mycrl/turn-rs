/// slice as u16.
///
/// # Unit Test
///
/// ```
/// let int = convert::as_u16(&[0x00, 0x04]);
/// assert_eq!(int, 4);
/// ```
#[rustfmt::skip]
#[inline(always)]
pub fn as_u16(buf: &[u8]) -> u16 {
    assert!(buf.len() >= 2);
    u16::from_be_bytes([
        buf[0], 
        buf[1]
    ])
}

/// slice as u32.
///
/// # Unit Test
///
/// ```
/// let int = convert::as_u32(&[
///     0x00, 0x00, 0x00, 0x04
/// ]);
/// 
/// assert_eq!(int, 4);
/// ```
#[rustfmt::skip]
#[inline(always)]
pub fn as_u32(buf: &[u8]) -> u32 {
    assert!(buf.len() >= 4);
    u32::from_be_bytes([
        buf[0], 
        buf[1],
        buf[2],
        buf[3]
    ])
}

/// slice as u64.
///
/// # Unit Test
///
/// ```
/// let int = convert::as_u64(&[
///     0x00, 0x00, 0x00, 0x00,
///     0x00, 0x00, 0x00, 0x04
/// ]);
/// 
/// assert_eq!(int, 4);
/// ```
#[rustfmt::skip]
#[inline(always)]
pub fn as_u64(buf: &[u8]) -> u64 {
    assert!(buf.len() >= 8);
    u64::from_be_bytes([
        buf[0], 
        buf[1],
        buf[2],
        buf[3],
        buf[4], 
        buf[5],
        buf[6],
        buf[7],
    ])
}