use std::convert::TryFrom;
use anyhow::ensure;
use super::util;

/// channel data message.
pub struct ChannelData<'a> {
    /// channnel data bytes.
    pub buf: &'a [u8],
    /// channel number.
    pub number: u16,
}

impl<'a> TryFrom<&'a [u8]> for ChannelData<'a> {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer: [u8; 4] = [
    ///     0x00, 0x01, 0x00, 0x00
    /// ];
    ///         
    /// let data = ChannelData::try_from(&buffer[..]).unwrap();
    /// assert_eq!(data.number, 1);
    /// ```
    #[rustfmt::skip]
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        let len = buf.len();
        ensure!(len >= 4, "data len < 4");
        let size = util::as_u16(&buf[2..4]) as usize;
        ensure!(size <= len - 4, "data body len < size");
        let number = util::as_u16(&buf[..2]);
        Ok(Self { number, buf })
    }
}
