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
    /// let mut buf = BytesMut::new();
    /// let old = MessageReader::try_from(&buffer[..]).unwrap();
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
    /// let old = MessageReader::try_from(&buffer[..]).unwrap();
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
    /// let mut buf = BytesMut::with_capacity(1280);
    /// let old = MessageReader::try_from(&buffer[..]).unwrap();
    /// let mut message = MessageWriter::derive(Kind::BindingRequest, &old, &mut buf);
    /// message.try_into(Some(&util::long_key("panda", "panda", "raspberry"))).unwrap();
    /// assert_eq!(&buf[..], &result);
    /// ```
    pub fn fold(&mut self, auth: Option<&Auth>) -> Result<&[u8]> {
        // write attribute list size.
        let size = (self.raw.len() - 20) as u16;
        let size_buf = size.to_be_bytes();
        self.raw[2] = size_buf[0];
        self.raw[3] = size_buf[1];

        // if need message integrity?
        if let Some(a) = auth {
            self.integrity(a)?;
        }
        
        Ok(self.raw)
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
    /// let mut buf = BytesMut::from(&buffer[..]);
    /// let old = MessageReader::try_from(&buffer[..]).unwrap();
    /// let mut message = MessageWriter::derive(Kind::BindingRequest, &old, &mut buf);
    /// message.try_into(Some(&util::long_key("panda", "panda", "raspberry"))).unwrap();
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
