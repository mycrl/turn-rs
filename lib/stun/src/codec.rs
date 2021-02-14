use std::convert::TryFrom;
use super::{
    util,
    AttrKind, 
    ChannelData, 
    Kind, 
    Message, 
    Property,
    Auth
};

use anyhow::{
    anyhow, 
    ensure, 
    Result
};

use bytes::{
    BufMut, 
    BytesMut
};

const ZOER_BUF: [u8; 10] = [0u8; 10];
const UNKNOWN_PAYLOAD: Message = Message {
    kind: Kind::Unknown,
    attributes: vec![],
    buffer: &[],
    token: &[],
    effective_block: 0,
};

/// decoder stun message
///
/// # Unit Test
///
/// ```
/// use stun::*;
/// use stun::codec::*;
/// 
/// let buffer: [u8; 20] = [
///     0x00, 0x01, 0x00, 0x00, 
///     0x21, 0x12, 0xa4, 0x42,
///     0x72, 0x6d, 0x49, 0x42, 
///     0x72, 0x52, 0x64, 0x48,
///     0x57, 0x62, 0x4b, 0x2b
/// ];
/// 
/// let token: [u8; 12] = [
///     0x72, 0x6d, 0x49, 0x42, 
///     0x72, 0x52, 0x64, 0x48, 
///     0x57, 0x62, 0x4b, 0x2b
/// ];
///         
/// let message = decode_message(&buffer).unwrap();
/// assert_eq!(message.kind, Kind::BindingRequest);
/// assert_eq!(message.token, token);
/// assert_eq!(message.get(AttrKind::UserName), None);
/// ```
#[rustfmt::skip]
pub fn decode_message<'a>(buffer: &'a [u8]) -> Result<Message<'a>> {
    ensure!(buffer.len() >= 20, "message len < 20");
    let count_size = buffer.len();
    let mut attributes = Vec::new();
    let mut find_effective_block = false;
    let mut effective_block = 0;

    // message type
    let kind = Kind::try_from(util::as_u16(&buffer[..2]))
        .unwrap_or(Kind::Unknown);
    
    // when the message type is not supported, 
    // directly return the undefined message type.
    if Kind::Unknown == kind {
        return Ok(UNKNOWN_PAYLOAD)
    }

    // message size
    // magic cookie
    let size = util::as_u16(&buffer[2..4]) as usize;
    let cookie = u32::from_be_bytes([
        buffer[4],
        buffer[5],
        buffer[6],
        buffer[7]
    ]);

    // check fixed cookie
    // check if the message size is overflow
    ensure!(cookie == 0x2112A442, "missing cookie");
    ensure!(count_size >= size + 20, "missing len");

    // get transaction id
    let token = &buffer[8..20];
    let mut offset = 20;
    
loop {

    // if the length is not long enough to continue, 
    // jump out of the loop.
    if count_size - offset < 4 {
        break;
    }

    // get attribute type
    let key = u16::from_be_bytes([
        buffer[offset],
        buffer[offset + 1]
    ]);

    // whether the MessageIntegrity attribute has been found, 
    // if found, record the current offset position.
    if !find_effective_block {
        effective_block = offset as u16;
    }

    // check whether the current attribute is MessageIntegrity, 
    // if it is, mark this attribute has been found.
    if key == AttrKind::MessageIntegrity as u16 {
        find_effective_block = true;
    }

    // get attribute size
    let size = u16::from_be_bytes([
        buffer[offset + 2],
        buffer[offset + 3]
    ]) as usize;

    // check if the attribute length has overflowed.
    offset += 4;
    if count_size - offset < size {
        break;
    }

    // get attribute body
    let value = &buffer[
        offset..
        offset + size
    ];

    // if there are padding bytes, 
    // skip padding size.
    let psize = util::pad_size(size);
    if size > 0 {
        offset += size + psize;
    }

    // skip the attributes that are not supported.
    let dyn_attribute = match AttrKind::try_from(key) {
        Err(_) => continue,
        Ok(a) => a
    };

    // insert attribute to attributes list.
    if let Ok(attribute) = dyn_attribute.from(token, value) {
        attributes.push((dyn_attribute, attribute));
    }
}

    Ok(Message {
        kind,
        token,
        buffer,
        attributes,
        effective_block,
    })
}

/// encoder stun message
///
/// # Unit Test
///
/// ```
/// use stun::codec::*;
/// use bytes::BytesMut;
/// 
/// let buffer: [u8; 20] = [
///     0x00, 0x01, 0x00, 0x00, 
///     0x21, 0x12, 0xa4, 0x42,
///     0x72, 0x6d, 0x49, 0x42, 
///     0x72, 0x52, 0x64, 0x48,
///     0x57, 0x62, 0x4b, 0x2b
/// ];
/// 
/// let mut buf = BytesMut::with_capacity(1280);
/// let msg = decode_message(&buffer).unwrap();
/// encode_message(msg, &mut buf, None).unwrap();
/// assert_eq!(&buf[..], &buffer);
/// ```
#[rustfmt::skip]
pub fn encode_message(message: Message, buf: &mut BytesMut, auth: Option<Auth>) -> Result<()> {
    assert_ne!(message.kind, Kind::Unknown);
    unsafe { buf.set_len(0) }
    
    // message type
    // message size
    // fixed cookie
    // transaction id
    buf.put_u16(message.kind as u16);
    buf.put_u16(0);
    buf.put_u32(0x2112A442);
    buf.put(message.token);
    
    // insert attribute list
for (k, v) in message.attributes {
    buf.put_u16(k as u16);

    // record the current position, 
    // and then advance the internal cursor 2 bytes, 
    // here is to reserve the position.
    let os = buf.len();
    unsafe { buf.advance_mut(2) }
    v.into_bytes(buf, message.token);

    // compute write index,
    // back to source index write size.
    let size = buf.len() - os - 2;
    let size_buf = (size as u16).to_be_bytes();
    buf[os] = size_buf[0];
    buf[os + 1] = size_buf[1];

    // if you need to padding, 
    // padding in the zero bytes.
    let psize = util::pad_size(size);
    if psize > 0 {
        buf.put(&ZOER_BUF[0..psize]);
    }
}
    
    // write attribute list size.
    let attr_size = (buf.len() - 20) as u16;
    let size_buf = attr_size.to_be_bytes();
    buf[2] = size_buf[0];
    buf[3] = size_buf[1];
    
    // if need message integrity?
    if let Some(a) = auth {
        append_integrity(buf, a)?;
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
/// use stun::codec::*;
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
/// append_integrity(&mut buf, ("panda", "panda", "raspberry")).unwrap();
/// assert_eq!(&buf[..], &result);
/// ```
#[rustfmt::skip]
pub fn append_integrity(buffer: &mut BytesMut, auth: Auth) -> Result<()> {
    assert!(buffer.len() >= 20);
    
    // compute new size,
    // new size include the MessageIntegrity attribute size.
    let mut buffer_size = (buffer.len() + 4) as u16;
    let size_buf = buffer_size.to_be_bytes();
    
    // overwrite old size with new size.
    buffer[2] = size_buf[0];
    buffer[3] = size_buf[1];
    
    // long key,
    // digest the message buffer,
    // create the new MessageIntegrity attribute.
    let key = util::long_key(auth.0, auth.1, auth.2);
    let hmac_output = util::hmac_sha1(&key, vec![&buffer])?.into_bytes();
    let property_buf = hmac_output.as_slice();

    // write MessageIntegrity attribute.
    buffer.put_u16(AttrKind::MessageIntegrity as u16);
    buffer.put_u16(20);
    buffer.put(property_buf);

    // compute new size,
    // new size include the Fingerprint attribute size.
    buffer_size += 8;
    let size_buf = buffer_size.to_be_bytes();

    // overwrite old size with new size.
    buffer[2] = size_buf[0];
    buffer[3] = size_buf[1];

    // CRC Fingerprint
    buffer.put_u16(AttrKind::Fingerprint as u16);
    buffer.put_u16(4);
    buffer.put_u32(util::fingerprint(&buffer));

    Ok(())
}

/// check MessageIntegrity attribute.
///
/// return whether the `MessageIntegrity` attribute 
/// contained in the message can pass the check.
///
/// # Unit Test
///
/// ```
/// use stun::codec::*;
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
/// let message = decode_message(&buffer).unwrap();
/// let result = assert_integrity(&message, ("panda", "panda", "raspberry")).unwrap();
/// assert!(result);
/// ```
#[rustfmt::skip]
pub fn assert_integrity(payload: &Message<'_>, auth: Auth) -> Result<bool> {
    assert!(!payload.buffer.is_empty());
    assert!(payload.effective_block > 20);

    // unwrap MessageIntegrity attribute,
    // an error occurs if not found.
    let integrity = payload
        .get(AttrKind::MessageIntegrity)
        .ok_or_else(|| anyhow!("not found MessageIntegrity"))?;

    // create multiple submit.
    let size_buf = (payload.effective_block + 4).to_be_bytes();
    let body = vec![
        &payload.buffer[0..2],
        &size_buf,
        &payload.buffer[4..payload.effective_block as usize]
    ];
    
    // digest the message buffer.
    let key = util::long_key(auth.0, auth.1, auth.2);
    let hmac_output = util::hmac_sha1(&key, body)?.into_bytes();
    let property_buf = hmac_output.as_slice();

    // Compare local and original attribute.
    Ok(match integrity {
        Property::MessageIntegrity(x) => &property_buf == x,
        _ => false
    })
}

/// decoder ChannelDate message.
///
/// # Unit Test
///
/// ```
/// use stun::codec::*;
/// 
/// let buffer = [
///     0x40u8, 0x00, 0x00, 0x1e, 
///     0x80, 0xcf, 0x00, 0x03, 
///     0x5b, 0xd2, 0x3c, 0x06, 
///     0x43, 0x4b, 0x13, 0xfe, 
///     0xb6, 0xc4, 0xa7, 0x85, 
///     0x80, 0x00, 0x00, 0x01, 
///     0x65, 0xe3, 0x98, 0x9a,
///     0x46, 0xe0, 0x45, 0x88, 
///     0x5d, 0x60
/// ];
/// 
/// let data = decode_channel(&buffer).unwrap();
/// assert_eq!(data.number, 0x4000);
/// ```
#[rustfmt::skip]
pub fn decode_channel(buf: &[u8]) -> Result<ChannelData<'_>> {
    let len = buf.len();
    ensure!(len >= 4, "data len < 4");
    let size = util::as_u16(&buf[2..4]) as usize;
    ensure!(size <= len - 4, "data body len < size");
    Ok(ChannelData {
        number: util::as_u16(&buf[..2]),
        buf,
    })
}