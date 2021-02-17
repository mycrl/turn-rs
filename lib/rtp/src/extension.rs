use std::convert::TryFrom;
use anyhow::ensure;
use bytes::{
    BytesMut,
    BufMut,
    Bytes,
    Buf
};

/// ### RTP Header Extension
/// 
/// ```bash
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |      defined by profile       |           length              |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |                        header extension                       |
///  |                             ....                              |
/// ```
#[derive(Debug, Clone)]
pub struct Extension {
    /// defined by profile.
    pub kind: u16,
    /// header extension list.
    pub data: Vec<u32>, 
}

impl Extension {
    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use rtp::extension::Extension;
    ///
    /// let buffer = [
    ///     0xbe, 0xde, 0x00, 0x02, 0x22, 0x5b, 0xb3, 0x33,
    ///     0x41, 0x00, 0x8b, 0x00
    /// ];
    /// 
    /// let mut writer = BytesMut::new();
    /// let extension = Extension {
    ///     data: vec![576434995, 1090554624],
    ///     kind: 48862,
    /// };
    /// 
    /// extension.into(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    pub fn into(self, buf: &mut BytesMut) {
        buf.put_u16(self.kind);
        buf.put_u16(self.data.len() as u16);
        for item in self.data {
            buf.put_u32(item);
        }
    }
}

impl<'a> TryFrom<&'a mut Bytes> for Extension {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use bytes::Bytes;
    /// use rtp::extension::Extension;
    /// use std::convert::TryFrom;
    ///
    /// let mut buffer = Bytes::from_static(&[
    ///     0xbe, 0xde, 0x00, 0x02, 0x22, 0x5b, 0xb3, 0x33,
    ///     0x41, 0x00, 0x8b, 0x00
    /// ]);
    /// 
    /// let extension = Extension::try_from(&mut buffer).unwrap();
    /// assert_eq!(extension.kind, 48862);
    /// assert_eq!(extension.data.len(), 2);
    /// assert_eq!(extension.data[0], 576434995);
    /// assert_eq!(extension.data[1], 1090554624);
    /// ```
    #[rustfmt::skip]
    fn try_from(buf: &'a mut Bytes) -> Result<Self, Self::Error> {
        ensure!(buf.len() >= 4, "buf len < 4");
        
        let kind = buf.get_u16();
        let count = buf.get_u16() as usize;
        
        let is_overflow = buf.len() >= count * 4;
        ensure!(is_overflow, "buf len is too short");
        
        let data = (0..count)
            .map(|_| buf.get_u32())
            .collect::<Vec<u32>>();
        
        Ok(Self {
            kind,
            data
        })
    }
}
