pub mod connection;
pub mod bandwidth;
pub mod origin;

use connection::Connection;
use origin::Origin;
use anyhow::{
    ensure,
    anyhow
};

use std::convert::{
    TryFrom,
    Into
};

/// Network type.
#[derive(Debug, PartialEq, Eq)]
pub enum NetKind {
    /// Internet
    IN,
}

/// Address type.
#[derive(Debug, PartialEq, Eq)]
pub enum AddrKind {
    /// Ipv4
    IP4,
    /// Ipv6
    IP6,
}

/// SDP: Session Description Protocol
///
/// An SDP description is denoted by the media type "application/sdp"
/// (See Section 8).
/// 
/// An SDP description is entirely textual.  SDP field names and
/// attribute names use only the US-ASCII subset of UTF-8 [RFC3629], but
/// textual fields and attribute values MAY use the full ISO 10646
/// character set in UTF-8 encoding, or some other character set defined
/// by the "a=charset:" attribute (Section 6.10).  Field and attribute
/// values that use the full UTF-8 character set are never directly
/// compared, hence there is no requirement for UTF-8 normalization.  The
/// textual form, as opposed to a binary encoding such as ASN.1 or XDR,
/// was chosen to enhance portability, to enable a variety of transports
/// to be used, and to allow flexible, text-based toolkits to be used to
/// generate and process session descriptions.  However, since SDP may be
/// used in environments where the maximum permissible size of a session
/// description is limited, the encoding is deliberately compact.  Also,
/// since descriptions may be transported via very unreliable means or
/// damaged by an intermediate caching server, the encoding was designed
/// with strict order and formatting rules so that most errors would
/// result in malformed session descriptions that could be detected
/// easily and discarded.
/// 
/// An SDP description consists of a number of lines of text of the form:
/// 
/// <type>=<value>
/// 
/// where <type> is exactly one case-significant character and <value> is
/// structured text whose format depends on <type>.  In general, <value>
/// is either a number of subfields delimited by a single space character
/// or a free format string, and is case-significant unless a specific
/// field defines otherwise.  Whitespace separators are not used on
/// either side of the "=" sign, however, the value can contain a leading
/// whitespace as part of its syntax, i.e., that whitespace is part of
/// the value.
#[derive(Debug)]
pub struct Sdp<'a> {
    /// Origin ("o=")
    pub origin: Option<Origin<'a>>,
    /// Session Name ("s=")
    /// The "s=" line (session-name-field) is the textual session name.
    /// There MUST be one and only one "s=" line per session description.
    /// The "s=" line MUST NOT be empty.  If a session has no meaningful
    /// name, then "s= " or "s=-" (i.e., a single space or dash as the
    /// session name) is RECOMMENDED.  If a session-level "a=charset:"
    /// attribute is present, it specifies the character set used in the "s="
    /// field.  If a session-level "a=charset:" attribute is not present, the
    /// "s=" field MUST contain ISO 10646 characters in UTF-8 encoding.
    pub session_name: Option<&'a str>,
    /// Session Information ("i=")
    /// The "i=" line (information-field) provides textual information about
    /// the session.  There can be at most one session-level "i=" line per
    /// session description, and at most one "i=" line in each media
    /// description.  Unless a media-level "i=" line is provided, the
    /// session-level "i=" line applies to that media description.  If the
    /// "a=charset:" attribute is present, it specifies the character set
    /// used in the "i=" line.  If the "a=charset:" attribute is not present,
    /// the "i=" line MUST contain ISO 10646 characters in UTF-8 encoding.
    /// 
    /// At most one "i=" line can be used for each media description.  In
    /// media definitions, "i=" lines are primarily intended for labeling
    /// media streams.  As such, they are most likely to be useful when a
    /// single session has more than one distinct media stream of the same
    /// media type.  An example would be two different whiteboards, one for
    /// slides and one for feedback and questions.
    /// 
    /// The "i=" line is intended to provide a free-form human-readable
    /// description of the session or the purpose of a media stream.  It is
    /// not suitable for parsing by automata.
    pub session_info: Option<&'a str>,
    /// URI ("u=")
    /// The "u=" line (uri-field) provides a URI (Uniform Resource
    /// Identifier) [RFC3986].  The URI should be a pointer to additional
    /// human readable information about the session.  This line is OPTIONAL.
    /// No more than one "u=" line is allowed per session description.
    pub uri: Option<&'a str>,
    /// Email Address and Phone Number ("e=" and "p=")
    /// The "e=" line (email-field) and "p=" line (phone-field) specify
    /// contact information for the person responsible for the session.  This
    /// is not necessarily the same person that created the session
    /// description.
    pub email: Option<&'a str>,
    pub phone: Option<&'a str>,
    /// Connection Information ("c=")
    pub connection: Option<Connection>,
    
}

impl Into<&'static str> for NetKind {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::NetKind;
    /// use std::convert::*;
    ///
    /// let kind: &'static str = NetKind::IN.into();
    /// assert_eq!(kind, "IN");
    /// ```
    fn into(self) -> &'static str {
        "IN"
    }
}

impl<'a> TryFrom<&'a str> for NetKind {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::NetKind;
    /// use std::convert::*;
    ///
    /// assert_eq!(NetKind::try_from("IN").unwrap(), NetKind::IN);
    /// assert_eq!(NetKind::try_from("in").is_ok(), false);
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        ensure!(value == "IN", "invalid nettype!");
        Ok(Self::IN)
    }
}

impl Into<&'static str> for AddrKind {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::AddrKind;
    /// use std::convert::*;
    ///
    /// let ipv4_kind: &'static str = AddrKind::IP4.into();
    /// let ipv6_kind: &'static str = AddrKind::IP6.into();
    /// assert_eq!(ipv4_kind, "IP4");
    /// assert_eq!(ipv6_kind, "IP6");
    /// ```
    fn into(self) -> &'static str {
        match self {
            Self::IP4 => "IP4",
            Self::IP6 => "IP6",
        }
    }
}

impl<'a> TryFrom<&'a str> for AddrKind {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::AddrKind;
    /// use std::convert::*;
    ///
    /// assert_eq!(AddrKind::try_from("IP4").unwrap(), AddrKind::IP4);
    /// assert_eq!(AddrKind::try_from("IP6").unwrap(), AddrKind::IP6);
    /// assert_eq!(AddrKind::try_from("ipv4").is_ok(), false);
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "IP4" => Ok(Self::IP4),
            "IP6" => Ok(Self::IP6),
            _ => Err(anyhow!("invalid addrtype!"))
        }
    }
}
