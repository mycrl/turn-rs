use std::convert::TryFrom;
use anyhow::{
    Result,
    ensure,
    anyhow
};

use super::attribute::{
    MessageIntegrity,
    AttrKind,
    Property
};

use super::{
    Kind,
    util
};

use bytes::{
    BytesMut,
    BufMut
};

const ZOER_BUF: [u8; 10] = [0u8; 10];
const COOKIE: [u8; 4] = 0x2112A442u32.to_be_bytes();

/// (username, password, realm)
type Auth = [u8; 16];

/// stun message writer.
pub struct MessageWriter<'a> {
    token: &'a [u8],
    raw: &'a mut BytesMut,
}

impl<'a, 'b> MessageWriter<'a> {
    /// rely on old message to create new message.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    ///   
    /// let mut attributes = Vec::new();
    /// let mut buf = BytesMut::new();
    /// let old = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// MessageWriter::derive(Kind::BindingRequest, &old, &mut buf);
    /// assert_eq!(&buf[..], &buffer[..]);
    /// ```
    #[rustfmt::skip]
    pub fn derive(
        kind: Kind, 
        reader: &MessageReader<'a, 'b>, 
        raw: &'a mut BytesMut
    ) -> Self {
        unsafe { raw.set_len(0) }
        raw.put_u16(kind as u16);
        raw.put_u16(0);
        raw.put(&COOKIE[..]);
        raw.put(reader.token);
        Self {
            raw,
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
    /// use stun::*;
    /// use stun::attribute::UserName;
    /// use std::convert::TryFrom;
    /// use bytes::BytesMut;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    /// 
    /// let new_buf = [
    ///     0x00u8, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b,
    ///     0x00, 0x06, 0x00, 0x05,
    ///     0x70, 0x61, 0x6e, 0x64,
    ///     0x61, 0x00, 0x00, 0x00
    /// ];
    ///
    /// let mut buf = BytesMut::new();
    /// let mut attributes = Vec::new();
    /// let old = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// let mut message = MessageWriter::derive(Kind::BindingRequest, &old, &mut buf);
    /// message.append::<UserName>("panda");
    /// assert_eq!(&new_buf[..], &buf[..]);
    /// ```
    #[rustfmt::skip]
    pub fn append<T: Property<'a>>(&mut self, value: T::Inner) {
        self.raw.put_u16(T::kind() as u16);
        
        // record the current position, 
        // and then advance the internal cursor 2 bytes, 
        // here is to reserve the position.
        let os = self.raw.len();
        unsafe { self.raw.advance_mut(2) }
        T::into(value, &mut self.raw, self.token);
        
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
    /// use stun::*;
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    /// 
    /// let result = [
    ///     0x00u8, 0x01, 0x00, 0x20,
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42,
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b,
    ///     0x00, 0x08, 0x00, 0x14,
    ///     0x45, 0x0e, 0x6e, 0x44,
    ///     0x52, 0x1e, 0xe8, 0xde,
    ///     0x2c, 0xf0, 0xfa, 0xb6,
    ///     0x9c, 0x5c, 0x19, 0x17,
    ///     0x98, 0xc6, 0xd9, 0xde, 
    ///     0x80, 0x28, 0x00, 0x04,
    ///     0xed, 0x41, 0xb6, 0xbe
    /// ];
    /// 
    /// let mut attributes = Vec::new();
    /// let mut buf = BytesMut::with_capacity(1280);
    /// let old = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// let mut message = MessageWriter::derive(Kind::BindingRequest, &old, &mut buf);
    /// message.fold(Some(&util::long_key("panda", "panda", "raspberry"))).unwrap();
    /// assert_eq!(&buf[..], &result);
    /// ```
    pub fn fold(&mut self, auth: Option<&Auth>) -> Result<()> {
        // write attribute list size.
        let size = (self.raw.len() - 20) as u16;
        let size_buf = size.to_be_bytes();
        self.raw[2] = size_buf[0];
        self.raw[3] = size_buf[1];

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
    /// use stun::*;
    /// use bytes::BytesMut;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    /// 
    /// let result = [
    ///     0x00u8, 0x01, 0x00, 0x20,
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42,
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b,
    ///     0x00, 0x08, 0x00, 0x14,
    ///     0x45, 0x0e, 0x6e, 0x44,
    ///     0x52, 0x1e, 0xe8, 0xde,
    ///     0x2c, 0xf0, 0xfa, 0xb6,
    ///     0x9c, 0x5c, 0x19, 0x17,
    ///     0x98, 0xc6, 0xd9, 0xde, 
    ///     0x80, 0x28, 0x00, 0x04,
    ///     0xed, 0x41, 0xb6, 0xbe
    /// ];
    /// 
    /// let mut attributes = Vec::new();
    /// let mut buf = BytesMut::from(&buffer[..]);
    /// let old = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// let mut message = MessageWriter::derive(Kind::BindingRequest, &old, &mut buf);
    /// message.fold(Some(&util::long_key("panda", "panda", "raspberry"))).unwrap();
    /// assert_eq!(&buf[..], &result);
    /// ```
    #[rustfmt::skip]
    fn integrity(&mut self, auth: &Auth) -> Result<()> {
        assert!(self.raw.len() >= 20);
        
        // compute new size,
        // new size include the MessageIntegrity attribute size.
        let mut buf_size = (self.raw.len() + 4) as u16;
        let size_buf = buf_size.to_be_bytes();

        // overwrite old size with new size.
        self.raw[2] = size_buf[0];
        self.raw[3] = size_buf[1];

        // long key,
        // digest the message buffer,
        // create the new MessageIntegrity attribute.
        let hmac_output = util::hmac_sha1(auth, vec![&self.raw])?.into_bytes();
        let property_buf = hmac_output.as_slice();

        // write MessageIntegrity attribute.
        self.raw.put_u16(AttrKind::MessageIntegrity as u16);
        self.raw.put_u16(20);
        self.raw.put(property_buf);

        // compute new size,
        // new size include the Fingerprint attribute size.
        buf_size += 8;
        let size_buf = buf_size.to_be_bytes();

        // overwrite old size with new size.
        self.raw[2] = size_buf[0];
        self.raw[3] = size_buf[1];

        // CRC Fingerprint
        self.raw.put_u16(AttrKind::Fingerprint as u16);
        self.raw.put_u16(4);
        self.raw.put_u32(util::fingerprint(&self.raw));

        Ok(())
    }
}

/// stun message reader.
pub struct MessageReader<'a, 'b> {
    /// message type.
    pub kind: Kind,
    /// message transaction id.
    pub token: &'a [u8],
    /// message source bytes.
    raw: &'a [u8],
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
    /// use stun::*;
    /// use stun::attribute::*;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    /// 
    /// let mut attributes = Vec::new();
    /// let message = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// assert!(message.get::<UserName>().is_none());
    /// ```
    #[rustfmt::skip]
    pub fn get<T: Property<'a>>(&self) -> Option<Result<T::Inner, T::Error>> {
        let kind = T::kind();
        self.attributes
            .iter()
            .find(|(k, _)| k == &kind)
            .map(|(_, v)| T::try_from(v, self.token))
    }
    
    /// check MessageReaderIntegrity attribute.
    /// 
    /// return whether the `MessageReaderIntegrity` attribute 
    /// contained in the message can pass the check.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer = [
    ///     0x00u8, 0x03, 0x00, 0x50, 
    ///     0x21, 0x12, 0xa4, 0x42, 
    ///     0x64, 0x4f, 0x5a, 0x78, 
    ///     0x6a, 0x56, 0x33, 0x62, 
    ///     0x4b, 0x52, 0x33, 0x31, 
    ///     0x00, 0x19, 0x00, 0x04, 
    ///     0x11, 0x00, 0x00, 0x00, 
    ///     0x00, 0x06, 0x00, 0x05, 
    ///     0x70, 0x61, 0x6e, 0x64, 
    ///     0x61, 0x00, 0x00, 0x00, 
    ///     0x00, 0x14, 0x00, 0x09, 
    ///     0x72, 0x61, 0x73, 0x70, 
    ///     0x62, 0x65, 0x72, 0x72, 
    ///     0x79, 0x00, 0x00, 0x00, 
    ///     0x00, 0x15, 0x00, 0x10, 
    ///     0x31, 0x63, 0x31, 0x33, 
    ///     0x64, 0x32, 0x62, 0x32, 
    ///     0x34, 0x35, 0x62, 0x33, 
    ///     0x61, 0x37, 0x33, 0x34, 
    ///     0x00, 0x08, 0x00, 0x14,
    ///     0xd6, 0x78, 0x26, 0x99, 
    ///     0x0e, 0x15, 0x56, 0x15, 
    ///     0xe5, 0xf4, 0x24, 0x74, 
    ///     0xe2, 0x3c, 0x26, 0xc5, 
    ///     0xb1, 0x03, 0xb2, 0x6d
    /// ];
    /// 
    /// let mut attributes = Vec::new();
    /// let message = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// let result = message.integrity(&util::long_key("panda", "panda", "raspberry")).is_ok();
    /// assert!(result);
    /// ```
    #[rustfmt::skip]
    pub fn integrity(&self, auth: &Auth) -> Result<()> {
        ensure!(!self.raw.is_empty(), "buf is empty");
        ensure!(self.valid_offset > 20, "buf is empty");

        // unwrap MessageIntegrity attribute,
        // an error occurs if not found.
        let integrity = self
            .get::<MessageIntegrity>()
            .ok_or_else(|| anyhow!("not found MessageIntegrity"))??;

        // create multiple submit.
        let size_buf = (self.valid_offset + 4).to_be_bytes();
        let body = vec![
            &self.raw[0..2],
            &size_buf,
            &self.raw[4..self.valid_offset as usize]
        ];

        // digest the message buffer.
        let hmac_output = util::hmac_sha1(auth, body)?.into_bytes();
        let property_buf = hmac_output.as_slice();

        // Compare local and original attribute.
        if integrity != property_buf {
            return Err(anyhow!("assert fail!"))
        }

        Ok(())
    }

    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
    /// use stun::attribute::*;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer: [u8; 20] = [
    ///     0x00, 0x01, 0x00, 0x00, 
    ///     0x21, 0x12, 0xa4, 0x42,
    ///     0x72, 0x6d, 0x49, 0x42, 
    ///     0x72, 0x52, 0x64, 0x48,
    ///     0x57, 0x62, 0x4b, 0x2b
    /// ];
    /// 
    /// let mut attributes = Vec::new();
    /// let message = MessageReader::decode(&buffer[..], &mut attributes).unwrap();
    /// assert_eq!(message.kind, Kind::BindingRequest);
    /// assert!(message.get::<UserName>().is_none());
    /// ```
    pub fn decode(buf: &'a [u8], attributes: &'b mut Vec<(AttrKind, &'a [u8])>) -> Result<MessageReader<'a, 'b>> {
        ensure!(buf.len() >= 20, "message len < 20");
        let mut find_valid_offset = false;
        let mut valid_offset = 0;
        let count_size = buf.len();

        // message type
        // message size
        // check fixed magic cookie
        // check if the message size is overflow
        let kind = Kind::try_from(util::as_u16(&buf[..2]))?;
        let size = util::as_u16(&buf[2..4]) as usize;
        ensure!(buf[4..8] == COOKIE[..], "missing cookie");
        ensure!(count_size >= size + 20, "missing len");

        // get transaction id
        let token = &buf[8..20];
        let mut offset = 20;
        
    loop {

        // if the buf length is not long enough to continue, 
        // jump out of the loop.
        if count_size - offset < 4 {
            break;
        }

        // get attribute type
        let key = u16::from_be_bytes([
            buf[offset],
            buf[offset + 1]
        ]);

        // whether the MessageIntegrity attribute has been found, 
        // if found, record the current offset position.
        if !find_valid_offset {
            valid_offset = offset as u16;
        }

        // check whether the current attribute is MessageIntegrity, 
        // if it is, mark this attribute has been found.
        if key == AttrKind::MessageIntegrity as u16 {
            find_valid_offset = true;
        }

        // skip the attributes that are not supported.
        let attrkind = match AttrKind::try_from(key) {
            Err(_) => continue,
            Ok(a) => a
        };

        // get attribute size
        let size = u16::from_be_bytes([
            buf[offset + 2],
            buf[offset + 3]
        ]) as usize;

        // check if the attribute length has overflowed.
        offset += 4;
        if count_size - offset < size {
            break;
        }

        // get attribute body
        // insert attribute to attributes list.
        attributes.push((attrkind, &buf[
            offset..
            offset + size
        ]));

        // if there are padding bytes, 
        // skip padding size.
        let psize = util::pad_size(size);
        if size > 0 {
            offset += size + psize;
        }
    }

        Ok(Self {
            kind,
            token,
            raw: buf,
            attributes,
            valid_offset,
        })
    }
}
