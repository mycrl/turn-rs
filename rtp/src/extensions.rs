use std::convert::TryFrom;
use anyhow::ensure;
use bytes::{
    BytesMut,
    BufMut,
    Buf
};

const HEAD_MASK: u8 = 0b11110000;
const LE_HEAD_MASK: u8 = 0b00001111;

/// ### One-Byte Header
///
/// Each extension element starts with a byte containing an ID and a
/// length:
///
/// 0
/// 0 1 2 3 4 5 6 7
/// +-+-+-+-+-+-+-+-+
/// |  ID   |  len  |
/// +-+-+-+-+-+-+-+-+
/// 
/// 
/// The 4-bit ID is the local identifier of this element in the range
/// 1-14 inclusive.  In the signaling section, this is referred to as the
/// valid range.
/// 
/// The local identifier value 15 is reserved for future extension and
/// MUST NOT be used as an identifier.  If the ID value 15 is
/// encountered, its length field should be ignored, processing of the
/// entire extension should terminate at that point, and only the
/// extension elements present prior to the element with ID 15
/// considered.
/// 
/// The 4-bit length is the number minus one of data bytes of this header
/// extension element following the one-byte header.  Therefore, the
/// value zero in this field indicates that one byte of data follows, and
/// a value of 15 (the maximum) indicates element data of 16 bytes.
/// (This permits carriage of 16-byte values, which is a common length of
/// labels and identifiers, while losing the possibility of zero-length
/// values -- which would often be padded anyway.
#[derive(Debug, Clone)]
pub struct Extension<'a> {
    pub kind: u8,
    pub data: &'a [u8],
}

impl<'a> Extension<'a> {
    /// # Unit Test
    ///
    /// ```
    /// use rtp::extensions::Extension;
    /// use std::convert::TryFrom;
    ///
    /// let mut buffer = [
    ///     0x22, 0xaa, 0x36, 0x3f
    /// ];
    /// 
    /// let extension = Extension::try_from(&buffer[..]).unwrap();
    /// assert_eq!(extension.len(), 4);
    /// ```
    pub fn len(&self) -> usize {
        self.data.len() + 1
    }

    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use rtp::extensions::Extension;
    ///
    /// let buffer = [
    ///     0x22, 0xaa, 0x36, 0x3f
    /// ];
    /// 
    /// let mut writer = BytesMut::new();
    /// let extension = Extension {
    ///     data: &[0xaa, 0x36, 0x3f],
    ///     kind: 2,
    /// };
    /// 
    /// extension.into(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    pub fn into(self, buf: &mut BytesMut) {
        let size = self.data.len() as u8 - 1;
        let mut head = 0u8;

        head = (head & LE_HEAD_MASK) | (self.kind << 4);
        head = (head & HEAD_MASK) | size;

        buf.put_u8(head);
        buf.put(&self.data[..])
    }
}

impl<'a> TryFrom<&'a [u8]> for Extension<'a> {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use rtp::extensions::Extension;
    /// use std::convert::TryFrom;
    ///
    /// let mut buffer = [
    ///     0x22, 0xaa, 0x36, 0x3f
    /// ];
    /// 
    /// let extension = Extension::try_from(&buffer[..]).unwrap();
    /// assert_eq!(extension.kind, 2);
    /// assert_eq!(extension.data.len(), 3);
    /// assert_eq!(extension.data, &[0xaa, 0x36, 0x3f]);
    /// ```
    #[rustfmt::skip]
    fn try_from(mut buf: &'a [u8]) -> Result<Self, Self::Error> {
        ensure!(buf.len() >= 1, "buf len < 1");
        
        let head = buf.get_u8();
        let kind = (head & HEAD_MASK) >> 4;
        let size = ((head & LE_HEAD_MASK) + 1) as usize;
        ensure!(buf.len() >= size, "buf len is too short");
        
        Ok(Self {
            kind,
            data: &buf[..size],
        })
    }
}

/// ### RTP Header Extension
///
/// In the one-byte header form of extensions, the 16-bit value required
/// by the RTP specification for a header extension, labeled in the RTP
/// specification as "defined by profile", takes the fixed bit pattern
/// 0xBEDE (the first version of this specification was written on the
/// feast day of the Venerable Bede).
/// 
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
pub struct Extensions<'a>(
    pub Vec<Extension<'a>>
);

impl<'a> Extensions<'a> {
    /// # Unit Test
    ///
    /// ```
    /// use rtp::extensions::Extensions;
    /// use std::convert::TryFrom;
    ///
    /// let buffer = [
    ///     0xbe, 0xde, 0x00, 0x01, 
    ///     0x22, 0xaa, 0x36, 0x3f
    /// ];
    /// 
    /// let extensions = Extensions::try_from(&buffer[..]).unwrap();
    /// assert_eq!(extensions.len(), 8);
    /// ```
    pub fn len(&self) -> usize {
        4 + self.0.iter().map(|e| e.len()).sum::<usize>()
    }

    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use rtp::extensions::*;
    ///
    /// let buffer = [
    ///     0xbe, 0xde, 0x00, 0x01, 
    ///     0x22, 0xaa, 0x36, 0x3f
    /// ];
    /// 
    /// let mut writer = BytesMut::new();
    /// let extensions = Extensions(vec![
    ///     Extension {
    ///         data: &[0xaa, 0x36, 0x3f],
    ///         kind: 2,
    ///     } 
    /// ]);
    /// 
    /// extensions.into(&mut writer);
    /// assert_eq!(&writer[..], &buffer[..]);
    /// ```
    pub fn into(self, buf: &mut BytesMut) {
        buf.put_u16(0xBEDE);
        buf.put_u16(self.0.len() as u16);

        for extension in self.0 {
            extension.into(buf);
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for Extensions<'a> {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use rtp::extensions::Extensions;
    /// use std::convert::TryFrom;
    ///
    /// let buffer = [
    ///     0xbe, 0xde, 0x00, 0x01, 
    ///     0x22, 0xaa, 0x36, 0x3f
    /// ];
    /// 
    /// let extensions = Extensions::try_from(&buffer[..]).unwrap();
    /// assert_eq!(extensions.0.len(), 1);
    /// assert_eq!(extensions.0[0].kind, 2);
    /// ```
    #[rustfmt::skip]
    fn try_from(mut buf: &'a [u8]) -> Result<Self, Self::Error> {
        ensure!(buf.len() >= 4, "buf len < 4");
        
        let defined_by_profile = buf.get_u16();
        ensure!(defined_by_profile == 0xBEDE, "invalid rtp extension!");

        let size = buf.get_u16() as usize;
        let mut extensions = Vec::with_capacity(size);
        
        for _ in 0..size {
            extensions.push(Extension::try_from(buf)?);
        }
        
        Ok(Self(
            extensions
        ))
    }
}
