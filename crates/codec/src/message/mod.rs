pub mod attributes;
pub mod methods;

use crate::{
    Attributes, Error,
    crypto::{Password, fingerprint, hmac_sha1},
    message::{
        attributes::{Attribute, AttributeType, MessageIntegrity, MessageIntegritySha256},
        methods::Method,
    },
};

use bytes::{BufMut, BytesMut};

static MAGIC_NUMBER: u32 = 0x2112A442;

pub struct MessageEncoder<'a> {
    token: &'a [u8],
    bytes: &'a mut BytesMut,
}

impl<'a> MessageEncoder<'a> {
    pub fn new(method: Method, token: &'a [u8; 12], bytes: &'a mut BytesMut) -> Self {
        bytes.clear();
        bytes.put_u16(method.into());
        bytes.put_u16(0);
        bytes.put_u32(MAGIC_NUMBER);
        bytes.put(token.as_slice());

        Self { bytes, token }
    }

    /// rely on old message to create new message.
    ///
    /// # Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// use turn_server_codec::message::methods::*;
    /// use turn_server_codec::message::*;
    /// use turn_server_codec::*;
    ///
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49,
    ///     0x42, 0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let mut attributes = Attributes::default();
    /// let mut buf = BytesMut::new();
    /// let old = Message::decode(&buffer[..], &mut attributes).unwrap();
    /// MessageEncoder::extend(Method::Binding(MethodType::Request), &old, &mut buf);
    ///
    /// assert_eq!(&buf[..], &buffer[..]);
    /// ```
    pub fn extend(method: Method, reader: &Message<'a>, bytes: &'a mut BytesMut) -> Self {
        let token = reader.token();

        bytes.clear();
        bytes.put_u16(method.into());
        bytes.put_u16(0);
        bytes.put_u32(MAGIC_NUMBER);
        bytes.put(token);
        Self { bytes, token }
    }

    /// append attribute.
    ///
    /// append attribute to message attribute list.
    ///
    /// # Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// use turn_server_codec::message::attributes::*;
    /// use turn_server_codec::message::methods::*;
    /// use turn_server_codec::message::*;
    /// use turn_server_codec::*;
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
    /// let mut attributes = Attributes::default();
    /// let old = Message::decode(&buffer[..], &mut attributes).unwrap();
    /// let mut message =
    ///     MessageEncoder::extend(Method::Binding(MethodType::Request), &old, &mut buf);
    ///
    /// message.append::<UserName>("panda");
    ///
    /// assert_eq!(&new_buf[..], &buf[..]);
    /// ```
    pub fn append<'c, T: Attribute<'c>>(&'c mut self, value: T::Item) {
        self.bytes.put_u16(T::TYPE as u16);

        // record the current position,
        // and then advance the internal cursor 2 bytes,
        // here is to reserve the position.
        let os = self.bytes.len();
        unsafe { self.bytes.advance_mut(2) }
        T::serialize(value, self.bytes, self.token);

        // compute write index,
        // back to source index write size.
        let size = self.bytes.len() - os - 2;
        let size_buf = (size as u16).to_be_bytes();
        self.bytes[os] = size_buf[0];
        self.bytes[os + 1] = size_buf[1];

        // if you need to padding,
        // padding in the zero bytes.
        let psize = alignment_32(size);
        if psize > 0 {
            self.bytes.put(&[0u8; 10][0..psize]);
        }
    }

    /// try decoder bytes as message.
    ///
    /// # Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// use turn_server_codec::message::methods::*;
    /// use turn_server_codec::message::*;
    /// use turn_server_codec::*;
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
    /// let mut attributes = Attributes::default();
    /// let mut buf = BytesMut::with_capacity(1280);
    /// let old = Message::decode(&buffer[..], &mut attributes).unwrap();
    /// let mut message =
    ///     MessageEncoder::extend(Method::Binding(MethodType::Request), &old, &mut buf);
    ///
    /// message
    ///     .flush(Some(&turn_server_codec::crypto::generate_password(
    ///         "panda",
    ///         "panda",
    ///         "raspberry",
    ///         turn_server_codec::message::attributes::PasswordAlgorithm::Md5,
    ///     )))
    ///     .unwrap();
    ///
    /// assert_eq!(&buf[..], &result);
    /// ```
    pub fn flush(&mut self, password: Option<&Password>) -> Result<(), Error> {
        // write attribute list size.
        self.set_len(self.bytes.len() - 20);

        // if need message integrity?
        if let Some(it) = password {
            self.checksum(it)?;
        }

        Ok(())
    }

    /// append MessageIntegrity attribute.
    ///
    /// add the `MessageIntegrity` attribute to the stun message
    /// and serialize the message into a buffer.
    ///
    /// # Test
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// use turn_server_codec::message::methods::*;
    /// use turn_server_codec::message::*;
    /// use turn_server_codec::*;
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
    /// let mut attributes = Attributes::default();
    /// let mut buf = BytesMut::from(&buffer[..]);
    /// let old = Message::decode(&buffer[..], &mut attributes).unwrap();
    /// let mut message =
    ///     MessageEncoder::extend(Method::Binding(MethodType::Request), &old, &mut buf);
    ///
    /// message
    ///     .flush(Some(&turn_server_codec::crypto::generate_password(
    ///         "panda",
    ///         "panda",
    ///         "raspberry",
    ///         turn_server_codec::message::attributes::PasswordAlgorithm::Md5,
    ///     )))
    ///     .unwrap();
    ///
    /// assert_eq!(&buf[..], &result);
    /// ```
    fn checksum(&mut self, passwrd: &Password) -> Result<(), Error> {
        assert!(self.bytes.len() >= 20);
        let len = self.bytes.len();

        // compute new size,
        // new size include the MessageIntegrity attribute size.
        self.set_len(len + 4);

        // write MessageIntegrity attribute.
        {
            let hmac = hmac_sha1(passwrd, &[self.bytes]);
            self.bytes.put_u16(match passwrd {
                Password::Md5(_) => AttributeType::MessageIntegrity as u16,
                Password::Sha256(_) => AttributeType::MessageIntegritySha256 as u16,
            });

            self.bytes.put_u16(20);
            self.bytes.put(hmac.as_slice());
        }

        // compute new size,
        // new size include the Fingerprint attribute size.
        self.set_len(len + 4 + 8);

        // CRC Fingerprint
        let fingerprint = fingerprint(self.bytes);
        self.bytes.put_u16(AttributeType::Fingerprint as u16);
        self.bytes.put_u16(4);
        self.bytes.put_u32(fingerprint);

        Ok(())
    }

    // set stun message header size.
    fn set_len(&mut self, len: usize) {
        self.bytes[2..4].copy_from_slice((len as u16).to_be_bytes().as_slice());
    }
}

pub struct Message<'a> {
    /// message method.
    method: Method,
    /// message source bytes.
    bytes: &'a [u8],
    /// message payload size.
    size: u16,
    // message attribute list.
    attributes: &'a Attributes,
}

impl<'a> Message<'a> {
    /// message method.
    #[inline]
    pub fn method(&self) -> Method {
        self.method
    }

    /// message transaction id.
    #[inline]
    pub fn token(&self) -> &'a [u8] {
        &self.bytes[8..20]
    }

    /// get attribute.
    ///
    /// get attribute from message attribute list.
    ///
    /// # Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use turn_server_codec::message::attributes::*;
    /// use turn_server_codec::message::methods::*;
    /// use turn_server_codec::message::*;
    /// use turn_server_codec::*;
    ///
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49,
    ///     0x42, 0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let mut attributes = Attributes::default();
    /// let message = Message::decode(&buffer[..], &mut attributes).unwrap();
    ///
    /// assert!(message.get::<UserName>().is_none());
    /// ```
    pub fn get<T: Attribute<'a>>(&self) -> Option<T::Item> {
        let range = self.attributes.get(&T::TYPE)?;
        T::deserialize(&self.bytes[range], self.token()).ok()
    }

    /// Gets all the values of an attribute from a list.
    ///
    /// Normally a stun message can have multiple attributes with the same name,
    /// and this function will all the values of the current attribute.
    ///
    /// # Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use turn_server_codec::message::attributes::*;
    /// use turn_server_codec::message::methods::*;
    /// use turn_server_codec::message::*;
    /// use turn_server_codec::*;
    ///
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49,
    ///     0x42, 0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let mut attributes = Attributes::default();
    /// let message = Message::decode(&buffer[..], &mut attributes).unwrap();
    ///
    /// assert_eq!(message.get_all::<UserName>().next(), None);
    /// ```
    pub fn get_all<T: Attribute<'a>>(&self) -> impl Iterator<Item = T::Item> {
        self.attributes
            .get_all(&T::TYPE)
            .map(|it| T::deserialize(&self.bytes[it.clone()], self.token()))
            .filter(|it| it.is_ok())
            .flatten()
    }

    /// check MessageRefIntegrity attribute.
    ///
    /// return whether the `MessageRefIntegrity` attribute contained in the message
    /// can pass the check.
    ///
    ///
    /// # Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use turn_server_codec::message::methods::*;
    /// use turn_server_codec::message::*;
    /// use turn_server_codec::*;
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
    /// let mut attributes = Attributes::default();
    /// let message = Message::decode(&buffer[..], &mut attributes).unwrap();
    /// let result = message
    ///     .checksum(&turn_server_codec::crypto::generate_password(
    ///         "panda",
    ///         "panda",
    ///         "raspberry",
    ///         turn_server_codec::message::attributes::PasswordAlgorithm::Md5,
    ///     ))
    ///     .is_ok();
    ///
    /// assert!(result);
    /// ```
    pub fn checksum(&self, password: &Password) -> Result<(), Error> {
        if self.bytes.is_empty() || self.size < 20 {
            return Err(Error::InvalidInput);
        }

        // unwrap MessageIntegrity attribute,
        // an error occurs if not found.
        let integrity = match password {
            Password::Md5(_) => self.get::<MessageIntegrity>(),
            Password::Sha256(_) => self.get::<MessageIntegritySha256>(),
        }
        .ok_or(Error::NotFoundIntegrity)?;

        // create multiple submit.
        let size_buf = (self.size + 4).to_be_bytes();
        let body = [
            &self.bytes[0..2],
            &size_buf,
            &self.bytes[4..self.size as usize],
        ];

        // digest the message buffer.
        {
            // Compare local and original attribute.
            if integrity != hmac_sha1(password, &body).as_slice() {
                return Err(Error::IntegrityFailed);
            }
        }

        Ok(())
    }

    /// # Test
    ///
    /// ```
    /// use std::convert::TryFrom;
    /// use turn_server_codec::message::attributes::*;
    /// use turn_server_codec::message::methods::*;
    /// use turn_server_codec::message::*;
    /// use turn_server_codec::*;
    ///
    /// let buffer: [u8; 20] = [
    ///     0x00, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49, 0x42,
    ///     0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let mut attributes = Attributes::default();
    /// let message = Message::decode(&buffer[..], &mut attributes).unwrap();
    ///
    /// assert_eq!(
    ///     message.method(),
    ///     Method::Binding(MethodType::Request)
    /// );
    ///
    /// assert!(message.get::<UserName>().is_none());
    /// ```
    pub fn decode(bytes: &'a [u8], attributes: &'a mut Attributes) -> Result<Self, Error> {
        let len = bytes.len();

        // There must be at least a complete header.
        if len < 20 {
            return Err(Error::InvalidInput);
        }

        let method = Method::try_from(u16::from_be_bytes(bytes[..2].try_into()?))?;

        // First check whether the message length is valid. Here, the length needs
        // to add the 20 bytes of the header, because the length field here does
        // not include the header length.
        {
            let size = u16::from_be_bytes(bytes[2..4].try_into()?) as usize + 20;
            if len < size {
                return Err(Error::InvalidInput);
            }
        }

        // Check whether the magic number is the same.
        if bytes[4..8] != MAGIC_NUMBER.to_be_bytes() {
            return Err(Error::NotFoundMagicNumber);
        }

        let mut find_integrity = false;
        let mut content_len = 0;
        let mut offset = 20;

        loop {
            // if the buf length is not long enough to continue,
            // jump out of the loop.
            if len - offset < 4 {
                break;
            }

            // get attribute type
            let key = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);

            // whether the MessageIntegrity attribute has been found,
            // if found, record the current offset position.
            if !find_integrity {
                content_len = offset as u16;
            }

            // get attribute size
            let size = u16::from_be_bytes([bytes[offset + 2], bytes[offset + 3]]) as usize;

            // check if the attribute length has overflowed.
            offset += 4;
            if len - offset < size {
                break;
            }

            // body range.
            let range = offset..(offset + size);

            // if there are padding bytes,
            // skip padding size.
            if size > 0 {
                offset += size + alignment_32(size);
            }

            // skip the attributes that are not supported.
            let attrkind = if let Ok(kind) = AttributeType::try_from(key) {
                // check whether the current attribute is MessageIntegrity,
                // if it is, mark this attribute has been found.
                if kind == AttributeType::MessageIntegrity {
                    find_integrity = true;
                }

                kind
            } else {
                continue;
            };

            // get attribute body
            // insert attribute to attributes list.
            attributes.append(attrkind, range);
        }

        Ok(Self {
            size: content_len,
            attributes,
            method,
            bytes,
        })
    }

    /// # Test
    ///
    /// ```
    /// use turn_server_codec::message::*;
    ///
    /// let buffer: [u8; 20] = [
    ///     0x00, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42, 0x72, 0x6d, 0x49, 0x42,
    ///     0x72, 0x52, 0x64, 0x48, 0x57, 0x62, 0x4b, 0x2b,
    /// ];
    ///
    /// let size = Message::message_size(&buffer[..]).unwrap();
    ///
    /// assert_eq!(size, 20);
    /// ```
    pub fn message_size(buffer: &[u8]) -> Result<usize, Error> {
        if buffer[0] >> 6 != 0 || buffer.len() < 20 {
            return Err(Error::InvalidInput);
        }

        Ok((u16::from_be_bytes(buffer[2..4].try_into()?) + 20) as usize)
    }
}

/// compute padding size.
///
/// RFC5766 stipulates that the attribute content is a multiple of 4.
///
/// # Test
///
/// ```
/// use turn_server_codec::message::alignment_32;
///
/// assert_eq!(alignment_32(4), 0);
/// assert_eq!(alignment_32(0), 0);
/// assert_eq!(alignment_32(5), 3);
/// ```
#[inline(always)]
pub fn alignment_32(size: usize) -> usize {
    let range = size % 4;
    if size == 0 || range == 0 {
        return 0;
    }

    4 - range
}
