//! ## Session Traversal Utilities for NAT (STUN)
//!
//! [RFC8445]: https://tools.ietf.org/html/rfc8445
//! [RFC5626]: https://tools.ietf.org/html/rfc5626
//! [Section 13]: https://tools.ietf.org/html/rfc8489#section-13
//!
//! STUN is intended to be used in the context of one or more NAT
//! traversal solutions.  These solutions are known as "STUN Usages".
//! Each usage describes how STUN is utilized to achieve the NAT
//! traversal solution.  Typically, a usage indicates when STUN messages
//! get sent, which optional attributes to include, what server is used,
//! and what authentication mechanism is to be used.  Interactive
//! Connectivity Establishment (ICE) [RFC8445] is one usage of STUN.
//! SIP Outbound [RFC5626] is another usage of STUN.  In some cases,
//! a usage will require extensions to STUN. A STUN extension can be
//! in the form of new methods, attributes, or error response codes.
//! More information on STUN Usages can be found in [Section 13].

pub mod channel_data;
pub mod crypto;
pub mod message;

use self::{
    channel_data::ChannelData,
    message::{Message, attributes::AttributeType},
};

use std::{array::TryFromSliceError, ops::Range, str::Utf8Error};

#[derive(Debug)]
pub enum Error {
    InvalidInput,
    SummaryFailed,
    NotFoundIntegrity,
    IntegrityFailed,
    NotFoundMagicNumber,
    UnknownMethod,
    FatalError,
    Utf8Error(Utf8Error),
    TryFromSliceError(TryFromSliceError),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<Utf8Error> for Error {
    fn from(value: Utf8Error) -> Self {
        Self::Utf8Error(value)
    }
}

impl From<TryFromSliceError> for Error {
    fn from(value: TryFromSliceError) -> Self {
        Self::TryFromSliceError(value)
    }
}

pub enum DecodeResult<'a> {
    Message(Message<'a>),
    ChannelData(ChannelData<'a>),
}

/// A cache of the list of attributes, this is for internal use only.
#[derive(Debug, Clone)]
pub struct Attributes(Vec<(AttributeType, Range<usize>)>);

impl Default for Attributes {
    fn default() -> Self {
        Self(Vec::with_capacity(20))
    }
}

impl Attributes {
    /// Adds an attribute to the list.
    pub fn append(&mut self, kind: AttributeType, range: Range<usize>) {
        self.0.push((kind, range));
    }

    /// Gets an attribute from the list.
    ///
    /// Note: This function will only look for the first matching property in
    /// the list and return it.
    pub fn get(&self, kind: &AttributeType) -> Option<Range<usize>> {
        self.0
            .iter()
            .find(|(k, _)| k == kind)
            .map(|(_, v)| v.clone())
    }

    /// Gets all the values of an attribute from a list.
    ///
    /// Normally a stun message can have multiple attributes with the same name,
    /// and this function will all the values of the current attribute.
    pub fn get_all<'a>(
        &'a self,
        kind: &'a AttributeType,
    ) -> impl Iterator<Item = &'a Range<usize>> {
        self.0
            .iter()
            .filter(move |(k, _)| k == kind)
            .map(|(_, v)| v)
    }

    pub fn clear(&mut self) {
        if !self.0.is_empty() {
            self.0.clear();
        }
    }
}

#[derive(Default)]
pub struct Decoder(Attributes);

impl Decoder {
    /// # Test
    ///
    /// ```
    /// use turn_server::codec::message::attributes::UserName;
    /// use turn_server::codec::{Decoder, DecodeResult};
    ///
    /// let buffer = [
    ///     0x00, 0x01, 0x00, 0x4c, 0x21, 0x12, 0xa4, 0x42, 0x71, 0x66, 0x46, 0x31,
    ///     0x2b, 0x59, 0x79, 0x65, 0x56, 0x69, 0x32, 0x72, 0x00, 0x06, 0x00, 0x09,
    ///     0x55, 0x43, 0x74, 0x39, 0x3a, 0x56, 0x2f, 0x2b, 0x2f, 0x00, 0x00, 0x00,
    ///     0xc0, 0x57, 0x00, 0x04, 0x00, 0x00, 0x03, 0xe7, 0x80, 0x29, 0x00, 0x08,
    ///     0x22, 0x49, 0xda, 0x28, 0x2c, 0x6f, 0x2e, 0xdb, 0x00, 0x24, 0x00, 0x04,
    ///     0x6e, 0x00, 0x28, 0xff, 0x00, 0x08, 0x00, 0x14, 0x19, 0x58, 0xda, 0x38,
    ///     0xed, 0x1e, 0xdd, 0xc8, 0x6b, 0x8e, 0x22, 0x63, 0x3a, 0x22, 0x63, 0x97,
    ///     0xcf, 0xf5, 0xde, 0x82, 0x80, 0x28, 0x00, 0x04, 0x56, 0xf7, 0xa3, 0xed,
    /// ];
    ///
    /// let mut decoder = Decoder::default();
    /// let payload = decoder.decode(&buffer).unwrap();
    ///
    /// if let DecodeResult::Message(reader) = payload {
    ///     assert!(reader.get::<UserName>().is_some())
    /// }
    /// ```
    pub fn decode<'a>(&'a mut self, bytes: &'a [u8]) -> Result<DecodeResult<'a>, Error> {
        assert!(bytes.len() >= 4);

        let flag = bytes[0] >> 6;
        if flag > 3 {
            return Err(Error::InvalidInput);
        }

        Ok(if flag == 0 {
            self.0.clear();

            DecodeResult::Message(Message::decode(bytes, &mut self.0)?)
        } else {
            DecodeResult::ChannelData(ChannelData::decode(bytes)?)
        })
    }

    /// # Test
    ///
    /// ```
    /// use turn_server::codec::Decoder;
    ///
    /// let buffer = [
    ///     0x00, 0x01, 0x00, 0x4c, 0x21, 0x12, 0xa4, 0x42, 0x71, 0x66, 0x46, 0x31,
    ///     0x2b, 0x59, 0x79, 0x65, 0x56, 0x69, 0x32, 0x72, 0x00, 0x06, 0x00, 0x09,
    ///     0x55, 0x43, 0x74, 0x39, 0x3a, 0x56, 0x2f, 0x2b, 0x2f, 0x00, 0x00, 0x00,
    ///     0xc0, 0x57, 0x00, 0x04, 0x00, 0x00, 0x03, 0xe7, 0x80, 0x29, 0x00, 0x08,
    ///     0x22, 0x49, 0xda, 0x28, 0x2c, 0x6f, 0x2e, 0xdb, 0x00, 0x24, 0x00, 0x04,
    ///     0x6e, 0x00, 0x28, 0xff, 0x00, 0x08, 0x00, 0x14, 0x19, 0x58, 0xda, 0x38,
    ///     0xed, 0x1e, 0xdd, 0xc8, 0x6b, 0x8e, 0x22, 0x63, 0x3a, 0x22, 0x63, 0x97,
    ///     0xcf, 0xf5, 0xde, 0x82, 0x80, 0x28, 0x00, 0x04, 0x56, 0xf7, 0xa3, 0xed,
    /// ];
    ///
    /// let size = Decoder::message_size(&buffer, false).unwrap();
    ///
    /// assert_eq!(size, 96);
    /// ```
    pub fn message_size(bytes: &[u8], is_tcp: bool) -> Result<usize, Error> {
        let flag = bytes[0] >> 6;
        if flag > 3 {
            return Err(Error::InvalidInput);
        }

        Ok(if flag == 0 {
            Message::message_size(bytes)?
        } else {
            ChannelData::message_size(bytes, is_tcp)?
        })
    }
}

