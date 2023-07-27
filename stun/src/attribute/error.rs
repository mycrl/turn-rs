use crate::util;

use anyhow::ensure;
use bytes::{BufMut, BytesMut};
use num_enum::TryFromPrimitive;

use std::cmp::{Eq, PartialEq};
use std::convert::{Into, TryFrom};

/// The following error codes, along with their recommended reason
/// phrases, are defined:
///
/// 300  Try Alternate: The client should contact an alternate server for
///      this request.  This error response MUST only be sent if the
///      request included either a USERNAME or USERHASH attribute and a
///      valid MESSAGE-INTEGRITY or MESSAGE-INTEGRITY-SHA256 attribute;
///      otherwise, it MUST NOT be sent and error code 400 (Bad Request)
///      is suggested.  This error response MUST be protected with the
///      MESSAGE-INTEGRITY or MESSAGE-INTEGRITY-SHA256 attribute, and
///      receivers MUST validate the MESSAGE-INTEGRITY or MESSAGE-
///      INTEGRITY-SHA256 of this response before redirecting themselves
///      to an alternate server.
///      Note: Failure to generate and validate message integrity for a
///      300 response allows an on-path attacker to falsify a 300
///      response thus causing subsequent STUN messages to be sent to a
///      victim.
///      
/// 400  Bad Request: The request was malformed.  The client SHOULD NOT
///      retry the request without modification from the previous
///      attempt.  The server may not be able to generate a valid
///      MESSAGE-INTEGRITY or MESSAGE-INTEGRITY-SHA256 for this error, so
///      the client MUST NOT expect a valid MESSAGE-INTEGRITY or MESSAGE-
///      INTEGRITY-SHA256 attribute on this response.
///      
/// 401  Unauthenticated: The request did not contain the correct
///      credentials to proceed.  The client should retry the request
///      with proper credentials.
///      
/// 420  Unknown Attribute: The server received a STUN packet containing
///      a comprehension-required attribute that it did not understand.
///      The server MUST put this unknown attribute in the UNKNOWN-
///      ATTRIBUTE attribute of its error response.
///      
/// 438  Stale Nonce: The NONCE used by the client was no longer valid.
///      The client should retry, using the NONCE provided in the
///      response.
///      
/// 500  Server Error: The server has suffered a temporary error.  The
///      client should try again.
#[repr(u16)]
#[derive(TryFromPrimitive, PartialEq, Eq, Copy, Clone, Debug)]
pub enum Kind {
    TryAlternate = 0x0300,
    BadRequest = 0x0400,
    Unauthorized = 0x0401,
    Forbidden = 0x0403,
    RequestTimedout = 0x0408,
    UnknownAttribute = 0x0414,
    AllocationMismatch = 0x0425,
    StaleNonce = 0x0426,
    AddressFamilyNotSupported = 0x0428,
    WrongCredentials = 0x0429,
    UnsupportedTransportAddress = 0x042A,
    AllocationQuotaReached = 0x0456,
    ServerError = 0x0500,
    InsufficientCapacity = 0x0508,
}

/// [RFC3629]: https://datatracker.ietf.org/doc/html/rfc3629
/// [RFC7231]: https://datatracker.ietf.org/doc/html/rfc7231
/// [RFC3261]: https://datatracker.ietf.org/doc/html/rfc3261
/// [RFC3629]: https://datatracker.ietf.org/doc/html/rfc3629
///
/// The ERROR-CODE attribute is used in error response messages.  It
/// contains a numeric error code value in the range of 300 to 699 plus a
/// textual reason phrase encoded in UTF-8 [RFC3629]; it is also
/// consistent in its code assignments and semantics with SIP [RFC3261]
/// and HTTP [RFC7231].  The reason phrase is meant for diagnostic
/// purposes and can be anything appropriate for the error code.
/// Recommended reason phrases for the defined error codes are included
/// in the IANA registry for error codes.  The reason phrase MUST be a
/// UTF-8-encoded [RFC3629] sequence of fewer than 128 characters (which
/// can be as long as 509 bytes when encoding them or 763 bytes when
/// decoding them).
///
/// ```text
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |           Reserved, should be 0         |Class|     Number    |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |      Reason Phrase (variable)                                ..
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///
///              Figure 7: Format of ERROR-CODE Attribute
/// ```
///
/// To facilitate processing, the class of the error code (the hundreds
/// digit) is encoded separately from the rest of the code, as shown in
/// Figure 7.
///
/// The Reserved bits SHOULD be 0 and are for alignment on 32-bit
/// boundaries.  Receivers MUST ignore these bits.  The Class represents
/// the hundreds digit of the error code.  The value MUST be between 3
/// and 6.  The Number represents the binary encoding of the error code
/// modulo 100, and its value MUST be between 0 and 99.
#[derive(Clone, Debug)]
pub struct Error<'a> {
    pub code: u16,
    pub message: &'a str,
}

impl Error<'_> {
    /// create error from error type.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use faster_stun::attribute::*;
    ///
    /// Error::from(ErrKind::TryAlternate);
    /// ```
    pub fn from(code: Kind) -> Self {
        Self {
            code: code as u16,
            message: code.into(),
        }
    }

    /// encode the error type as bytes.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use faster_stun::attribute::*;
    /// use bytes::BytesMut;
    ///
    /// let buffer = [
    ///     0x00u8, 0x00, 0x03, 0x00, 0x54, 0x72, 0x79, 0x20, 0x41, 0x6c, 0x74, 0x65, 0x72, 0x6e, 0x61,
    ///     0x74, 0x65,
    /// ];
    ///
    /// let mut buf = BytesMut::with_capacity(1280);
    /// let error = Error::from(ErrKind::TryAlternate);
    /// error.into(&mut buf);
    /// assert_eq!(&buf[..], &buffer);
    /// ```
    pub fn into(self, buf: &mut BytesMut) {
        buf.put_u16(0x0000);
        buf.put_u16(self.code);
        buf.put(self.message.as_bytes());
    }
}

impl<'a> TryFrom<&'a [u8]> for Error<'a> {
    type Error = anyhow::Error;

    /// # Unit Test
    ///
    /// ```
    /// use faster_stun::attribute::*;
    /// use std::convert::TryFrom;
    ///
    /// let buffer = [
    ///     0x00u8, 0x00, 0x03, 0x00, 0x54, 0x72, 0x79, 0x20, 0x41, 0x6c, 0x74, 0x65, 0x72, 0x6e, 0x61,
    ///     0x74, 0x65,
    /// ];
    ///
    /// let error = Error::try_from(&buffer[..]).unwrap();
    /// assert_eq!(error.code, ErrKind::TryAlternate as u16);
    /// assert_eq!(error.message, "Try Alternate");
    /// ```
    fn try_from(packet: &'a [u8]) -> Result<Self, Self::Error> {
        ensure!(packet.len() >= 4, "buffer len < 4");
        ensure!(util::as_u16(&packet[..2]) == 0x0000, "missing reserved");
        Ok(Self {
            code: util::as_u16(&packet[2..4]),
            message: std::str::from_utf8(&packet[4..])?,
        })
    }
}

impl From<Kind> for &'static str {
    /// # Unit Test
    ///
    /// ```
    /// use faster_stun::attribute::*;
    /// use std::convert::Into;
    ///
    /// let err: &'static str = ErrKind::TryAlternate.into();
    /// assert_eq!(err, "Try Alternate");
    /// ```
    #[rustfmt::skip]
    fn from(val: Kind) -> Self {
        match val {
            Kind::TryAlternate => "Try Alternate",
            Kind::BadRequest => "Bad Request",
            Kind::Unauthorized => "Unauthorized",
            Kind::Forbidden => "Forbidden",
            Kind::RequestTimedout => "Request Timed out",
            Kind::UnknownAttribute => "Unknown Attribute",
            Kind::AllocationMismatch => "Allocation Mismatch",
            Kind::StaleNonce => "Stale Nonce",
            Kind::AddressFamilyNotSupported => "Address Family not Supported",
            Kind::WrongCredentials => "Wrong Credentials",
            Kind::UnsupportedTransportAddress => "Unsupported Transport Address",
            Kind::AllocationQuotaReached => "Allocation Quota Reached",
            Kind::ServerError => "Server Error",
            Kind::InsufficientCapacity => "Insufficient Capacity",
        }
    }
}

impl Eq for Error<'_> {}
impl PartialEq for Error<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code
    }
}
