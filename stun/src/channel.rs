use std::convert::TryFrom;
use anyhow::ensure;
use crate::util;

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
    /// use faster_stun::*;
    /// use std::convert::TryFrom;
    ///
    /// let buffer: [u8; 4] = [0x00, 0x01, 0x00, 0x00];
    ///
    /// let data = ChannelData::try_from(&buffer[..]).unwrap();
    /// assert_eq!(data.number, 1);
    /// ```
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        ensure!(buf.len() >= 4, "data len < 4");

        let size = util::as_u16(&buf[2..4]) as usize;
        ensure!(size <= buf.len() - 4, "data body len < size");

        Ok(Self {
            number: util::as_u16(&buf[..2]),
            buf,
        })
    }
}
