use bytes::{BufMut, BytesMut};

use super::Error;

/// The ChannelData Message
///
/// The ChannelData message is used to carry application data between the
/// client and the server.
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
    pub bytes: &'a [u8],
    pub number: u16,
}

impl<'a> ChannelData<'a> {
    pub fn number(&self) -> u16 {
        self.number
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.bytes
    }

    /// # Test
    ///
    /// ```
    /// use bytes::{BufMut, BytesMut};
    /// use turn_codec::channel_data::ChannelData;
    ///
    /// let data: [u8; 4] = [0x40, 0x00, 0x00, 0x40];
    /// let mut bytes = BytesMut::with_capacity(1500);
    ///
    /// ChannelData {
    ///     number: 16384,
    ///     bytes: &data[..],
    /// }
    /// .encode(&mut bytes);
    ///
    /// let size = ChannelData::message_size(&bytes[..], false).unwrap();
    ///
    /// assert_eq!(size, 8);
    /// ```
    pub fn message_size(bytes: &[u8], is_tcp: bool) -> Result<usize, Error> {
        if bytes.len() < 4 {
            return Err(Error::InvalidInput);
        }

        if !(1..3).contains(&(bytes[0] >> 6)) {
            return Err(Error::InvalidInput);
        }

        let mut size = (u16::from_be_bytes(bytes[2..4].try_into()?) + 4) as usize;
        if is_tcp && (size % 4) > 0 {
            size += 4 - (size % 4);
        }

        Ok(size)
    }

    /// # Test
    ///
    /// ```
    /// use bytes::{BufMut, BytesMut};
    /// use turn_codec::channel_data::ChannelData;
    ///
    /// let data: [u8; 4] = [0x40, 0x00, 0x00, 0x40];
    /// let mut bytes = BytesMut::with_capacity(1500);
    ///
    /// ChannelData {
    ///     number: 16384,
    ///     bytes: &data[..],
    /// }
    /// .encode(&mut bytes);
    ///
    /// let ret = ChannelData::decode(&bytes[..]).unwrap();
    ///
    /// assert_eq!(ret.number, 16384);
    /// assert_eq!(ret.bytes, &data[..]);
    /// ```
    pub fn encode(self, bytes: &mut BytesMut) {
        unsafe { bytes.set_len(0) }
        bytes.put_u16(self.number);
        bytes.put_u16(self.bytes.len() as u16);
        bytes.extend_from_slice(self.bytes);
    }

    /// # Test
    ///
    /// ```
    /// use bytes::{BufMut, BytesMut};
    /// use turn_codec::channel_data::ChannelData;
    ///
    /// let data: [u8; 4] = [0x40, 0x00, 0x00, 0x40];
    /// let mut bytes = BytesMut::with_capacity(1500);
    ///
    /// ChannelData {
    ///     number: 16384,
    ///     bytes: &data[..],
    /// }
    /// .encode(&mut bytes);
    ///
    /// let ret = ChannelData::decode(&bytes[..]).unwrap();
    ///
    /// assert_eq!(ret.number, 16384);
    /// assert_eq!(ret.bytes, &data[..]);
    /// ```
    pub fn decode(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < 4 {
            return Err(Error::InvalidInput);
        }

        let number = u16::from_be_bytes(bytes[..2].try_into()?);
        if !(0x4000..0xFFFF).contains(&number) {
            return Err(Error::InvalidInput);
        }

        let size = u16::from_be_bytes(bytes[2..4].try_into()?) as usize;
        if size > bytes.len() - 4 {
            return Err(Error::InvalidInput);
        }

        Ok(Self {
            bytes: &bytes[4..],
            number,
        })
    }
}
