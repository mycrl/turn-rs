pub mod attributes;
pub mod repeat_times;
pub mod time_zones;
pub mod connection;
pub mod bandwidth;
pub mod origin;
pub mod timing;
pub mod media;
pub mod util;

use repeat_times::RepeatTimes;
use time_zones::TimeZones;
use connection::Connection;
use bandwidth::Bandwidth;
use timing::Timing;
use origin::Origin;
use anyhow::{
    ensure,
    anyhow
};

use std::{
    convert::TryFrom,
    fmt
};

/// Sdp keys.
#[derive(Debug, PartialEq, Eq)]
pub enum Key {
    Origin,
    SessionName,
    SessionInfo,
    Uri,
    Email,
    Phone,
    Connection,
    Bandwidth,
    Timing,
    RepeatTimes,
    TimeZones,
}

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

/// media type.
/// 
/// <media> is the media type.  Currently defined media are "audio",
/// "video", "text", "application", and "message"
#[derive(Debug, PartialEq, Eq)]
pub enum Encoding {
    Audio,
    Video,
    Text,
    Application,
    Message
}

/// media proto.
///
/// <proto> is the transport protocol.  The meaning of the transport
/// protocol is dependent on the address type field in the relevant
/// "c=" field.  Thus a "c=" field of IP4 indicates that the transport
/// protocol runs over IP4.  The following transport protocols are
/// defined, but may be extended through registration of new protocols
/// with IANA (see [Section 8](https://datatracker.ietf.org/doc/html/rfc4566#section-8)):
/// 
/// *  udp: denotes an unspecified protocol running over UDP.
/// 
/// *  RTP/AVP: denotes RTP [19](https://datatracker.ietf.org/doc/html/rfc4566#ref-19) 
///    used under the RTP Profile for Audio and Video Conferences 
///    with Minimal Control [20](https://datatracker.ietf.org/doc/html/rfc4566#ref-20) 
///    running over UDP.
/// 
/// *  RTP/SAVP: denotes the Secure Real-time Transport Protocol 
///    [23](https://datatracker.ietf.org/doc/html/rfc4566#ref-23)
///    running over UDP.
///
/// The main reason to specify the transport protocol in addition to
/// the media format is that the same standard media formats may be
/// carried over different transport protocols even when the network
/// protocol is the same -- a historical example is vat Pulse Code
/// Modulation (PCM) audio and RTP PCM audio; another might be TCP/RTP
/// PCM audio.  In addition, relays and monitoring tools that are
/// transport-protocol-specific but format-independent are possible.
#[derive(Debug, PartialEq, Eq)]
pub enum Proto {
    Udp,
    Tls,
    Rtp,
    Avp,
    Savp
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
#[derive(Debug, Default)]
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
    /// Bandwidth ("b=")
    pub bandwidth: Option<Bandwidth>,
    /// Timing ("t=")
    pub timing: Option<Timing>,
    /// Repeat Times ("r=")
    pub repeat_times: Option<RepeatTimes>,
    /// Time Zones ("z=")
    pub time_zones: Option<TimeZones>
}

impl<'a> Sdp<'a> {
    pub fn handle_line(&mut self, key: Key, data: &'a str) -> anyhow::Result<()> {
        Ok(match key {
            Key::Origin => self.origin = Some(Origin::try_from(data)?),
            Key::SessionName => self.session_name = util::placeholder(data),
            Key::SessionInfo => self.session_info = util::placeholder(data),
            Key::Uri => self.uri = util::placeholder(data),
            Key::Email => self.email = util::placeholder(data),
            Key::Phone => self.phone = util::placeholder(data),
            Key::Connection => self.connection = Some(Connection::try_from(data)?),
            Key::Bandwidth => self.bandwidth = Some(Bandwidth::try_from(data)?),
            Key::Timing => self.timing = Some(Timing::try_from(data)?),
            Key::RepeatTimes => self.repeat_times = Some(RepeatTimes::try_from(data)?),
            Key::TimeZones => self.time_zones = Some(TimeZones::try_from(data)?),
        })
    }
}

impl<'a> TryFrom<&'a str> for Sdp<'a> {
    type Error = anyhow::Error;
    #[rustfmt::skip]
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let mut sdp = Self::default();
        for line in value.lines() {
            let (key, data) = line.split_at(2);
            sdp.handle_line(Key::try_from(key)?, data)?;
        }

        Ok(sdp)
    }
}

impl fmt::Display for NetKind {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::NetKind;
    ///
    /// assert_eq!(format!("{}", NetKind::IN), "IN");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IN")
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

impl fmt::Display for AddrKind {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::AddrKind;
    ///
    /// assert_eq!(format!("{}", AddrKind::IP4), "IP4");
    /// assert_eq!(format!("{}", AddrKind::IP6), "IP6");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::IP4 => "IP4",
            Self::IP6 => "IP6",
        })
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

impl fmt::Display for Key {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::Key;
    ///
    /// assert_eq!(format!("{}", Key::Origin), "o=");
    /// assert_eq!(format!("{}", Key::SessionName), "s=");
    /// assert_eq!(format!("{}", Key::SessionInfo), "i=");
    /// ```
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Origin =>          "o=",
            Self::SessionName =>     "s=",
            Self::SessionInfo =>     "i=",
            Self::Uri =>             "u=",
            Self::Email =>           "e=",
            Self::Phone =>           "p=",
            Self::Connection =>      "c=",
            Self::Bandwidth =>       "b=",
            Self::Timing =>          "t=",
            Self::RepeatTimes =>     "r=",
            Self::TimeZones =>       "z=",
        })
    }
}

impl<'a> TryFrom<&'a str> for Key {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::Key;
    /// use std::convert::*;
    ///
    /// let uri: Key = Key::try_from("u=").unwrap();
    /// let origin: Key = Key::try_from("o=").unwrap();
    /// let session_name: Key = Key::try_from("s=").unwrap();
    /// let session_info: Key = Key::try_from("i=").unwrap();
    ///
    /// assert_eq!(uri, Key::Uri);
    /// assert_eq!(origin, Key::Origin);
    /// assert_eq!(session_name, Key::SessionName);
    /// assert_eq!(session_info, Key::SessionInfo);
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "o=" => Ok(Self::Origin),
            "s=" => Ok(Self::SessionName),
            "i=" => Ok(Self::SessionInfo),
            "u=" => Ok(Self::Uri),
            "e=" => Ok(Self::Email),
            "p=" => Ok(Self::Phone),
            "c=" => Ok(Self::Connection),
            "b=" => Ok(Self::Bandwidth),
            "t=" => Ok(Self::Timing),
            "r=" => Ok(Self::RepeatTimes),
            "z=" => Ok(Self::TimeZones),
            _ => Err(anyhow!("invalid sdp key!"))
        }
    }
}

impl fmt::Display for Encoding {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::*;
    ///
    /// assert_eq!(format!("{}", Encoding::Audio), "audio");
    /// assert_eq!(format!("{}", Encoding::Video), "video");
    /// assert_eq!(format!("{}", Encoding::Text), "text");
    /// assert_eq!(format!("{}", Encoding::Application), "application");
    /// assert_eq!(format!("{}", Encoding::Message), "message");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Text => "text",
            Self::Application => "application",
            Self::Message => "message"
        })
    }
}

impl<'a> TryFrom<&'a str> for Encoding {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::*;
    /// use std::convert::TryFrom;
    ///
    /// assert_eq!(Encoding::try_from("text").unwrap(), Encoding::Text);
    /// assert_eq!(Encoding::try_from("audio").unwrap(), Encoding::Audio);
    /// assert_eq!(Encoding::try_from("video").unwrap(), Encoding::Video);
    /// assert_eq!(Encoding::try_from("message").unwrap(), Encoding::Message);
    /// assert_eq!(Encoding::try_from("application").unwrap(), Encoding::Application);
    /// assert!(Encoding::try_from("panda").is_err());
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "text" => Ok(Self::Text),
            "audio" => Ok(Self::Audio),
            "video" => Ok(Self::Video),
            "message" => Ok(Self::Message),
            "application" => Ok(Self::Application),
            _ => Err(anyhow!("invalid media type!"))
        }
    }
}

impl fmt::Display for Proto {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::*;
    ///
    /// assert_eq!(format!("{}", Proto::Udp), "UDP");
    /// assert_eq!(format!("{}", Proto::Tls), "TLS");
    /// assert_eq!(format!("{}", Proto::Rtp), "RTP");
    /// assert_eq!(format!("{}", Proto::Avp), "AVP");
    /// assert_eq!(format!("{}", Proto::Savp), "SAVP");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Udp => "UDP",
            Self::Tls => "TLS",
            Self::Rtp => "RTP",
            Self::Avp => "AVP",
            Self::Savp => "SAVP"
        })
    }
}

impl<'a> TryFrom<&'a str> for Proto {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::*;
    /// use std::convert::TryFrom;
    ///
    /// assert_eq!(Proto::try_from("UDP").unwrap(), Proto::Udp);
    /// assert_eq!(Proto::try_from("TLS").unwrap(), Proto::Tls);
    /// assert_eq!(Proto::try_from("RTP").unwrap(), Proto::Rtp);
    /// assert_eq!(Proto::try_from("AVP").unwrap(), Proto::Avp);
    /// assert_eq!(Proto::try_from("SAVP").unwrap(), Proto::Savp);
    /// assert!(Proto::try_from("udp").is_err());
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "UDP" => Ok(Self::Udp),
            "TLS" => Ok(Self::Tls),
            "RTP" => Ok(Self::Rtp),
            "AVP" => Ok(Self::Avp),
            "SAVP" => Ok(Self::Savp),
            _ => Err(anyhow!("invalid media proto!"))
        }
    }
}