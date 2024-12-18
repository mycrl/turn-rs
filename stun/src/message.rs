use bytes::{BufMut, BytesMut};

use std::convert::TryFrom;

use crate::StunError;

use super::attribute::{AttrKind, MessageIntegrity, Property};
use super::{util, Method};

const ZOER_BUF: [u8; 10] = [0u8; 10];
const COOKIE: [u8; 4] = 0x2112A442u32.to_be_bytes();

/// (username, password, realm)
type Auth = [u8; 16];

pub struct MessageWriter<'a> {
    token: &'a [u8],
    raw: &'a mut BytesMut,
}

impl<'a, 'b> MessageWriter<'a> {
    pub fn new(method: Method, token: &'a [u8; 12], buf: &'a mut BytesMut) -> Self {
        unsafe { buf.set_len(0) }
        buf.put_u16(method.into());
        buf.put_u16(0);
        buf.put(&COOKIE[..]);
        buf.put(token.as_slice());
        Self { raw: buf, token }
    }

    /// rely on old message to create new message.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// use stun::*;
    ///
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49,
    ///     0x42, 0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let mut attributes = Vec::new();
    /// let mut buf = BytesMut::new();
    /// let old = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// MessageWriter::extend(Method::Binding(Kind::Request), &old, &mut buf);
    /// assert_eq!(&buf[..], &buffer[..]);
    /// ```
    pub fn extend(method: Method, reader: &MessageReader<'a, 'b>, buf: &'a mut BytesMut) -> Self {
        unsafe { buf.set_len(0) }
        buf.put_u16(method.into());
        buf.put_u16(0);
        buf.put(&COOKIE[..]);
        buf.put(reader.token);
        Self {
            raw: buf,
            token: reader.token,
        }
    }

    /// append attribute.
    ///
    /// append attribute to message attribute list.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// use stun::attribute::UserName;
    /// use stun::*;
    ///
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49,
    ///     0x42, 0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let new_buf = [
    ///     0x00u8, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49,
    ///     0x42, 0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b, 0x00, 0x06, 0x00,
    ///     0x05, 0x70, 0x61, 0x6e, 0x64, 0x61, 0x00, 0x00, 0x00,
    /// ];
    ///
    /// let mut buf = BytesMut::new();
    /// let mut attributes = Vec::new();
    /// let old = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// let mut message =
    ///     MessageWriter::extend(Method::Binding(Kind::Request), &old, &mut buf);
    /// message.append::<UserName>("panda");
    /// assert_eq!(&new_buf[..], &buf[..]);
    /// ```
    pub fn append<T: Property<'a>>(&mut self, value: T::Inner) {
        self.raw.put_u16(T::kind() as u16);

        // record the current position,
        // and then advance the internal cursor 2 bytes,
        // here is to reserve the position.
        let os = self.raw.len();
        unsafe { self.raw.advance_mut(2) }
        T::into(value, self.raw, self.token);

        // compute write index,
        // back to source index write size.
        let size = self.raw.len() - os - 2;
        let size_buf = (size as u16).to_be_bytes();
        self.raw[os] = size_buf[0];
        self.raw[os + 1] = size_buf[1];

        // if you need to padding,
        // padding in the zero bytes.
        let psize = util::pad_size(size);
        if psize > 0 {
            self.raw.put(&ZOER_BUF[0..psize]);
        }
    }

    /// try decoder bytes as message.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// use stun::*;
    ///
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49,
    ///     0x42, 0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let result = [
    ///     0, 1, 0, 32, 33, 18, 164, 66, 114, 109, 73, 66, 114, 82, 100, 72, 87,
    ///     98, 75, 43, 0, 8, 0, 20, 69, 14, 110, 68, 82, 30, 232, 222, 44, 240,
    ///     250, 182, 156, 92, 25, 23, 152, 198, 217, 222, 128, 40, 0, 4, 74, 165,
    ///     171, 86,
    /// ];
    ///
    /// let mut attributes = Vec::new();
    /// let mut buf = BytesMut::with_capacity(1280);
    /// let old = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// let mut message =
    ///     MessageWriter::extend(Method::Binding(Kind::Request), &old, &mut buf);
    ///
    /// message
    ///     .flush(Some(&util::long_key("panda", "panda", "raspberry")))
    ///     .unwrap();
    /// assert_eq!(&buf[..], &result);
    /// ```
    pub fn flush(&mut self, auth: Option<&Auth>) -> Result<(), StunError> {
        // write attribute list size.
        self.set_len(self.raw.len() - 20);

        // if need message integrity?
        if let Some(a) = auth {
            self.integrity(a)?;
        }

        Ok(())
    }

    /// append MessageIntegrity attribute.
    ///
    /// add the `MessageIntegrity` attribute to the stun message
    /// and serialize the message into a buffer.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// use stun::*;
    ///
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49,
    ///     0x42, 0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let result = [
    ///     0, 1, 0, 32, 33, 18, 164, 66, 114, 109, 73, 66, 114, 82, 100, 72, 87,
    ///     98, 75, 43, 0, 8, 0, 20, 69, 14, 110, 68, 82, 30, 232, 222, 44, 240,
    ///     250, 182, 156, 92, 25, 23, 152, 198, 217, 222, 128, 40, 0, 4, 74, 165,
    ///     171, 86,
    /// ];
    ///
    /// let mut attributes = Vec::new();
    /// let mut buf = BytesMut::from(&buffer[..]);
    /// let old = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// let mut message =
    ///     MessageWriter::extend(Method::Binding(Kind::Request), &old, &mut buf);
    ///
    /// message
    ///     .flush(Some(&util::long_key("panda", "panda", "raspberry")))
    ///     .unwrap();
    /// assert_eq!(&buf[..], &result);
    /// ```
    fn integrity(&mut self, auth: &Auth) -> Result<(), StunError> {
        assert!(self.raw.len() >= 20);
        let len = self.raw.len();

        // compute new size,
        // new size include the MessageIntegrity attribute size.
        self.set_len(len + 4);

        // write MessageIntegrity attribute.
        let hmac_output = util::hmac_sha1(auth, &[self.raw])?.into_bytes();
        self.raw.put_u16(AttrKind::MessageIntegrity as u16);
        self.raw.put_u16(20);
        self.raw.put(hmac_output.as_slice());

        // compute new size,
        // new size include the Fingerprint attribute size.
        self.set_len(len + 4 + 8);

        // CRC Fingerprint
        let fingerprint = util::fingerprint(self.raw);
        self.raw.put_u16(AttrKind::Fingerprint as u16);
        self.raw.put_u16(4);
        self.raw.put_u32(fingerprint);

        Ok(())
    }

    // set stun message header size.
    fn set_len(&mut self, len: usize) {
        self.raw[2..4].copy_from_slice((len as u16).to_be_bytes().as_slice());
    }
}

#[derive(Debug)]
pub struct MessageReader<'a, 'b> {
    /// message type.
    pub method: Method,
    /// message transaction id.
    pub token: &'a [u8],
    /// message source bytes.
    buf: &'a [u8],
    /// message valid block bytes size.
    valid_offset: u16,
    // message attribute list.
    attributes: &'b Vec<(AttrKind, &'a [u8])>,
}

impl<'a, 'b> MessageReader<'a, 'b> {
    /// get attribute.
    ///
    /// get attribute from message attribute list.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use stun::attribute::*;
    /// use stun::*;
    ///
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49,
    ///     0x42, 0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let mut attributes = Vec::new();
    /// let message = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// assert!(message.get::<UserName>().is_none());
    /// ```
    pub fn get<T: Property<'a>>(&self) -> Option<T::Inner> {
        let kind = T::kind();
        self.attributes
            .iter()
            .find(|(k, _)| k == &kind)
            .and_then(|(_, v)| T::try_from(v, self.token).ok())
    }

    /// check MessageReaderIntegrity attribute.
    ///
    /// return whether the `MessageReaderIntegrity` attribute
    /// contained in the message can pass the check.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use stun::*;
    ///
    /// let buffer = [
    ///     0x00u8, 0x03, 0x00, 0x50, 0x21, 0x12, 0xa4, 0x42, 0x64, 0x4f, 0x5a,
    ///     0x78, 0x6a, 0x56, 0x33, 0x62, 0x4b, 0x52, 0x33, 0x31, 0x00, 0x19, 0x00,
    ///     0x04, 0x11, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x05, 0x70, 0x61, 0x6e,
    ///     0x64, 0x61, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x09, 0x72, 0x61, 0x73,
    ///     0x70, 0x62, 0x65, 0x72, 0x72, 0x79, 0x00, 0x00, 0x00, 0x00, 0x15, 0x00,
    ///     0x10, 0x31, 0x63, 0x31, 0x33, 0x64, 0x32, 0x62, 0x32, 0x34, 0x35, 0x62,
    ///     0x33, 0x61, 0x37, 0x33, 0x34, 0x00, 0x08, 0x00, 0x14, 0xd6, 0x78, 0x26,
    ///     0x99, 0x0e, 0x15, 0x56, 0x15, 0xe5, 0xf4, 0x24, 0x74, 0xe2, 0x3c, 0x26,
    ///     0xc5, 0xb1, 0x03, 0xb2, 0x6d,
    /// ];
    ///
    /// let mut attributes = Vec::new();
    /// let message = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// let result = message
    ///     .integrity(&util::long_key("panda", "panda", "raspberry"))
    ///     .is_ok();
    /// assert!(result);
    /// ```
    pub fn integrity(&self, auth: &Auth) -> Result<(), StunError> {
        if self.buf.is_empty() || self.valid_offset < 20 {
            return Err(StunError::InvalidInput);
        }

        // unwrap MessageIntegrity attribute,
        // an error occurs if not found.
        let integrity = self
            .get::<MessageIntegrity>()
            .ok_or(StunError::NotIntegrity)?;

        // create multiple submit.
        let size_buf = (self.valid_offset + 4).to_be_bytes();
        let body = [
            &self.buf[0..2],
            &size_buf,
            &self.buf[4..self.valid_offset as usize],
        ];

        // digest the message buffer.
        let hmac_output = util::hmac_sha1(auth, &body)?.into_bytes();
        let hmac_buf = hmac_output.as_slice();

        // Compare local and original attribute.
        if integrity != hmac_buf {
            return Err(StunError::IntegrityFailed);
        }

        Ok(())
    }

    /// # Unit Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use stun::attribute::*;
    /// use stun::*;
    ///
    /// let buffer: [u8; 20] = [
    ///     0x00, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49, 0x42,
    ///     0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let mut attributes = Vec::new();
    /// let message = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// assert_eq!(message.method, Method::Binding(Kind::Request));
    /// assert!(message.get::<UserName>().is_none());
    /// ```
    #[rustfmt::skip]
    pub fn decode(
        buf: &'a [u8],
        attributes: &'b mut Vec<(AttrKind, &'a [u8])>,
    ) -> Result<MessageReader<'a, 'b>, StunError> {
        if buf.len() < 20 {
            return Err(StunError::InvalidInput)
        }

        let mut find_integrity = false;
        let mut valid_offset = 0;
        let count_size = buf.len();

        // message type
        // message size
        // check fixed magic cookie
        // check if the message size is overflow
        let method = Method::try_from(util::as_u16(&buf[..2]))?;
        let size = util::as_u16(&buf[2..4]) as usize + 20;
        if buf[4..8] != COOKIE[..] {
            return Err(StunError::NotCookie)
        }

        if count_size < size {
            return Err(StunError::InvalidInput)
        }

        // get transaction id
        let token = &buf[8..20];
        let mut offset = 20;

    // warn: loop
    loop {
        // if the buf length is not long enough to continue,
        // jump out of the loop.
        if count_size - offset < 4 {
            break;
        }

        // get attribute type
        let key = u16::from_be_bytes([buf[offset], buf[offset + 1]]);

        // whether the MessageIntegrity attribute has been found,
        // if found, record the current offset position.
        if !find_integrity {
            valid_offset = offset as u16;
        }

        // check whether the current attribute is MessageIntegrity,
        // if it is, mark this attribute has been found.
        if key == AttrKind::MessageIntegrity as u16 {
            find_integrity = true;
        }

        // get attribute size
        let size =
            u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]) as usize;

        // check if the attribute length has overflowed.
        offset += 4;
        if count_size - offset < size {
            break;
        }

        // body range.
        let range = offset..(offset + size);

        // if there are padding bytes,
        // skip padding size.
        if size > 0 {
            offset += size;
            offset += util::pad_size(size);
        }

        // skip the attributes that are not supported.
        let attrkind = match AttrKind::try_from(key) {
            Err(_) => continue,
            Ok(a) => a,
        };

        // get attribute body
        // insert attribute to attributes list.
        attributes.push((attrkind, &buf[range]));
    }

        Ok(Self {
            buf,
            token,
            method,
            attributes,
            valid_offset,
        })
    }

    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
    ///
    /// let buffer: [u8; 20] = [
    ///     0x00, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49, 0x42,
    ///     0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let size = MessageReader::message_size(&buffer[..]).unwrap();
    /// assert_eq!(size, 20);
    /// ```
    pub fn message_size(buf: &[u8]) -> Result<usize, StunError> {
        if buf[0] >> 6 != 0 || buf.len() < 20 {
            return Err(StunError::InvalidInput);
        }

        Ok((util::as_u16(&buf[2..4]) + 20) as usize)
    }
}

impl<'a> AsRef<[u8]> for MessageReader<'a, '_> {
    fn as_ref(&self) -> &'a [u8] {
        self.buf
    }
}

impl<'a> std::ops::Deref for MessageReader<'a, '_> {
    type Target = [u8];

    fn deref(&self) -> &'a Self::Target {
        self.buf
    }
}
