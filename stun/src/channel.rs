use crate::util;
use anyhow::{ensure, Result};
use std::convert::TryFrom;

/// The ChannelData Message
///
/// The ChannelData message is used to carry application data between the
/// client and the server.  
/// It has the following format:
///
/// ```text
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |         Channel Number        |            Length             |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// /                       Application Data                        /
/// /                                                               /
/// |                                                               |
/// |                               +-------------------------------+
/// |                               |
/// +-------------------------------+
///
///                               Figure 5
/// ```
///
/// The Channel Number field specifies the number of the channel on which
/// the data is traveling, and thus, the address of the peer that is
/// sending or is to receive the data.
///
/// The Length field specifies the length in bytes of the application
/// data field (i.e., it does not include the size of the ChannelData
/// header).  Note that 0 is a valid length.
///
/// The Application Data field carries the data the client is trying to
/// send to the peer, or that the peer is sending to the client.
#[derive(Debug)]
pub struct ChannelData<'a> {
    /// channnel data bytes.
    pub buf: &'a [u8],
    /// channel number.
    pub number: u16,
}

impl ChannelData<'_> {
    /// # Unit Test
    ///
    /// ```
    /// use faster_stun::*;
    /// use std::convert::TryFrom;
    ///
    /// let buffer: [u8; 4] = [0x40, 0x00, 0x00, 0x40];
    ///
    /// let size = ChannelData::message_size(&buffer[..], false).unwrap();
    /// assert_eq!(size, 68);
    /// ```
    pub fn message_size(buf: &[u8], is_tcp: bool) -> Result<usize> {
        ensure!(buf.len() >= 4, "data len < 4");
        ensure!((1..3).contains(&(buf[0] >> 6)), "not a channel data");
        let mut size = (util::as_u16(&buf[2..4]) + 4) as usize;
        if is_tcp && (size % 4) > 0 {
            size += 4 - (size % 4);
        }

        Ok(size)
    }
}

impl<'a> TryFrom<&'a [u8]> for ChannelData<'a> {
    type Error = anyhow::Error;

    /// # Unit Test
    ///
    /// ```
    /// use faster_stun::*;
    /// use std::convert::TryFrom;
    ///
    /// let buffer: [u8; 4] = [0x40, 0x00, 0x00, 0x00];
    ///
    /// let data = ChannelData::try_from(&buffer[..]).unwrap();
    /// assert_eq!(data.number, 16384);
    /// ```
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        ensure!(buf.len() >= 4, "data len < 4");
        let number = util::as_u16(&buf[..2]);
        ensure!((0x4000..0xFFFF).contains(&number), "invalid channel data");
        let size = util::as_u16(&buf[2..4]) as usize;
        ensure!(size <= buf.len() - 4, "data body len < size");
        Ok(Self { buf, number })
    }
}

impl AsRef<[u8]> for ChannelData<'_> {
    fn as_ref(&self) -> &[u8] {
        self.buf
    }
}

impl std::ops::Deref for ChannelData<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.buf
    }
}
