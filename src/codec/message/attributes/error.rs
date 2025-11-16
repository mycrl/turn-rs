use num_enum::TryFromPrimitive;

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
const fn errno(code: u16) -> u16 {
    ((code / 100) << 8) | (code % 100)
}

#[repr(u16)]
#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash, TryFromPrimitive)]
pub enum ErrorType {
    TryAlternate = errno(300),
    BadRequest = errno(400),
    Unauthorized = errno(401),
    Forbidden = errno(403),
    UnknownAttribute = errno(420),
    AllocationMismatch = errno(437),
    StaleNonce = errno(438),
    AddressFamilyNotSupported = errno(440),
    WrongCredentials = errno(441),
    UnsupportedTransportAddress = errno(442),
    PeerAddressFamilyMismatch = errno(443),
    AllocationQuotaReached = errno(486),
    ServerError = errno(500),
    InsufficientCapacity = errno(508),
}

impl From<ErrorType> for &'static str {
    /// # Test
    ///
    /// ```
    /// use std::convert::Into;
    /// use turn_server::codec::message::attributes::error::ErrorType;
    /// use turn_server::codec::Error;
    ///
    /// let err: &'static str = ErrorType::TryAlternate.into();
    /// assert_eq!(err, "Try Alternate");
    /// ```
    #[rustfmt::skip]
    fn from(val: ErrorType) -> Self {
        match val {
            ErrorType::TryAlternate => "Try Alternate",
            ErrorType::BadRequest => "Bad Request",
            ErrorType::Unauthorized => "Unauthorized",
            ErrorType::Forbidden => "Forbidden",
            ErrorType::UnknownAttribute => "Unknown Attribute",
            ErrorType::AllocationMismatch => "Allocation Mismatch",
            ErrorType::StaleNonce => "Stale Nonce",
            ErrorType::AddressFamilyNotSupported => "Address Family not Supported",
            ErrorType::WrongCredentials => "Wrong Credentials",
            ErrorType::UnsupportedTransportAddress => "Unsupported Transport Address",
            ErrorType::AllocationQuotaReached => "Allocation Quota Reached",
            ErrorType::ServerError => "Server Error",
            ErrorType::InsufficientCapacity => "Insufficient Capacity",
            ErrorType::PeerAddressFamilyMismatch => "Peer Address Family Mismatch",
        }
    }
}
