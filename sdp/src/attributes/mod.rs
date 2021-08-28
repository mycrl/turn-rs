mod rtp;
mod mid;
mod fmtp;
mod codec;
mod kind;
mod orient;
mod extension;

pub use orient::Orient;
pub use codec::Codec;
pub use extension::*;
pub use kind::Kind;
pub use mid::Mid;
pub use fmtp::*;
pub use rtp::*;

use super::util::*;
use anyhow::{
    Result,
    anyhow
};

use std::{
    convert::TryFrom,
    fmt
};

/// sdp attributes keys.
#[derive(Debug, PartialEq, Eq)]
pub enum Key {
    Fmtp,
    Lang,
    RtpMap,
    ExtMap,
    Charset,
    SdpLang,
    Ptime,
    MaxPtime,
    Orient,
    Type,
    Framerate,
    Quality
}

#[derive(Debug, Default)]
pub struct Attributes<'a> {
    /// ptime (Packet Time)
    /// 
    /// Name:  ptime
    /// Value:  ptime-value
    /// Usage Level:  media
    /// Charset Dependent:  no
    /// 
    /// Syntax:
    /// ptime-value = non-zero-int-or-real
    /// 
    /// Example:
    /// a=ptime:20
    /// 
    /// This gives the length of time in milliseconds represented by the
    /// media in a packet.  This is probably only meaningful for audio data,
    /// but may be used with other media types if it makes sense.  It should
    /// not be necessary to know "a=ptime:" to decode RTP or vat audio, and
    /// it is intended as a recommendation for the encoding/packetization of
    /// audio.
    pub ptime: Option<u64>,
    /// maxptime (Maximum Packet Time)
    /// 
    /// Name:  maxptime
    /// Value:  maxptime-value
    /// Usage Level:  media
    /// Charset Dependent:  no
    /// 
    /// Syntax:
    /// maxptime-value = non-zero-int-or-real
    /// 
    /// Example:
    /// a=maxptime:20
    /// 
    /// This gives the maximum amount of media that can be encapsulated in
    /// each packet, expressed as time in milliseconds.  The time SHALL be
    /// calculated as the sum of the time the media present in the packet
    /// represents.  For frame-based codecs, the time SHOULD be an integer
    /// multiple of the frame size.  This attribute is probably only
    /// meaningful for audio data, but may be used with other media types if
    /// it makes sense.  Note that this attribute was introduced after
    /// [RFC2327](https://datatracker.ietf.org/doc/html/rfc2327), 
    /// and implementations that have not been updated will ignore
    /// this attribute.
    pub maxptime: Option<u64>,
    /// Name:  rtpmap
    /// Value:  rtpmap-value
    /// Usage Level:  media
    /// Charset Dependent:  no

    /// Syntax:
    /// rtpmap-value = payload-type SP encoding-name
    /// "/" clock-rate [ "/" encoding-params ]
    /// payload-type = zero-based-integer
    /// encoding-name = token
    /// clock-rate = integer
    /// encoding-params = channels
    /// channels = integer
    pub rtpmap: RtpMap,
    /// Name:  fmtp
    /// Value:  fmtp-value
    /// Usage Level:  media
    /// Charset Dependent:  no
    /// 
    /// Syntax:
    /// fmtp-value = fmt SP format-specific-params
    /// format-specific-params = byte-string
    /// ; Notes:
    /// ; - The format parameters are media type parameters and
    /// ;   need to reflect their syntax.
    /// 
    /// Example:
    /// a=fmtp:96 profile-level-id=42e016;max-mbps=108000;max-fs=3600
    /// 
    /// This attribute allows parameters that are specific to a particular
    /// format to be conveyed in a way that SDP does not have to understand
    /// them.  The format must be one of the formats specified for the media.
    /// Format-specific parameters, semicolon separated, may be any set of
    /// parameters required to be conveyed by SDP and given unchanged to the
    /// media tool that will use this format.  At most one instance of this
    /// attribute is allowed for each format.
    /// 
    /// The "a=fmtp:" attribute may be used to specify parameters for any
    /// protocol and format that defines use of such parameters.
    pub fmtp: Fmtp<'a>,
    /// orient (Orientation)
    /// 
    /// Name:  orient
    /// Value:  orient-value
    /// Usage Level:  media
    /// 
    /// Charset Dependent:  no
    /// 
    /// Syntax:
    /// orient-value = portrait / landscape / seascape
    /// portrait  = %s"portrait"
    /// landscape = %s"landscape"
    /// seascape  = %s"seascape"
    /// ; NOTE: These names are case-sensitive.
    /// 
    /// Example:
    /// a=orient:portrait
    pub orient: Option<Orient>,
    /// Name:  charset
    /// Value:  charset-value
    /// Usage Level:  session
    /// Charset Dependent:  no
    /// 
    /// Syntax:
    /// charset-value = <defined in [RFC2978]>
    /// 
    /// This specifies the character set to be used to display the session
    /// name and information data.  By default, the ISO-10646 character set
    /// in UTF-8 encoding is used.  If a more compact representation is
    /// required, other character sets may be used.  For example, the ISO
    /// 8859-1 is specified with the following SDP attribute:
    /// 
    /// a=charset:ISO-8859-1
    /// 
    /// The charset specified MUST be one of those registered in the IANA
    /// Character Sets [registry](http://www.iana.org/assignments/character-
    /// sets), such as ISO-8859-1.  The character set identifier is a string
    /// that MUST be compared against identifiers from the "Name" or
    /// "Preferred MIME Name" field of the registry using a case-insensitive
    /// comparison.  If the identifier is not recognized or not supported,
    /// all strings that are affected by it SHOULD be regarded as octet
    /// strings.
    /// 
    /// Charset-dependent fields MUST contain only sequences of bytes that
    /// are valid according to the definition of the selected character set.
    /// Furthermore, charset-dependent fields MUST NOT contain the bytes 0x00
    /// (Nul), 0x0A (LF), and 0x0d (CR).
    pub charset: Option<&'a str>,
    /// Name:  sdplang
    /// Value:  sdplang-value
    /// Usage Level:  session, media
    /// Charset Dependent:  no
    /// 
    /// Syntax:
    /// sdplang-value = Language-Tag
    /// ; Language-Tag
    /// 
    /// Example:
    /// a=sdplang:fr
    /// 
    /// Multiple "a=sdplang:" attributes can be provided either at session or
    /// media level if the session description or media use multiple
    /// languages.
    /// 
    /// As a session-level attribute, it specifies the language for the
    /// session description (not the language of the media).  As a media-
    /// level attribute, it specifies the language for any media-level SDP
    /// information-field associated with that media (again not the language
    /// of the media), overriding any "a=sdplang:" attributes specified at
    /// session level.
    /// 
    /// In general, sending session descriptions consisting of multiple
    /// languages is discouraged.  Instead, multiple session descriptions
    /// SHOULD be sent describing the session, one in each language.
    /// However, this is not possible with all transport mechanisms, and so
    /// multiple "a=sdplang:" attributes are allowed although NOT
    /// RECOMMENDED.
    /// 
    /// The "a=sdplang:" attribute value must be a single language tag
    /// [RFC5646](https://datatracker.ietf.org/doc/html/rfc5646).  An 
    /// "a=sdplang:" attribute SHOULD be specified when a session is 
    /// distributed with sufficient scope to cross geographic boundaries, 
    /// where the language of recipients cannot be assumed, or where the 
    /// session is in a different language from the locally assumed norm.
    pub sdplang: Option<&'a str>,
    /// Name:  lang
    /// Value:  lang-value
    /// Usage Level:  session, media
    /// Charset Dependent:  no
    /// 
    /// Syntax:
    /// lang-value = Language-Tag
    /// ; Language-Tag
    /// 
    /// Example:
    /// a=lang:de
    /// 
    /// Multiple "a=lang:" attributes can be provided either at session or
    /// media level if the session or media has capabilities in more than one
    /// language, in which case the order of the attributes indicates the
    /// order of preference of the various languages in the session or media,
    /// from most preferred to least preferred.
    /// 
    /// As a session-level attribute, "a=lang:" specifies a language
    /// capability for the session being described.  As a media-level
    /// attribute, it specifies a language capability for that media,
    /// overriding any session-level language(s) specified.
    /// 
    /// The "a=lang:" attribute value must be a single [RFC5646](https://da
    /// tatracker.ietf.org/doc/html/rfc5646) language tag.  An "a=lang:" 
    /// attribute SHOULD be specified when a session is of sufficient scope 
    /// to cross geographic boundaries where the language of participants 
    /// cannot be assumed, or where the session has capabilities in languages 
    /// different from the locally assumed norm.
    /// 
    /// The "a=lang:" attribute is supposed to be used for setting the
    /// initial language(s) used in the session.  Events during the session
    /// may influence which language(s) are used, and the participants are
    /// not strictly bound to only use the declared languages.
    /// 
    /// Most real-time use cases start with just one language used, while
    /// other cases involve a range of languages, e.g., an interpreted or
    /// subtitled session.  When more than one "a=lang:" attribute is
    /// specified, the "a=lang:" attribute itself does not provide any
    /// information about multiple languages being intended to be used during
    /// the session, or if the intention is to only select one of the
    /// languages.  If needed, a new attribute can be defined and used to
    /// indicate such intentions.  Without such semantics, it is assumed that
    /// for a negotiated session one of the declared languages will be
    /// selected and used.
    pub lang: Option<&'a str>,
    /// Name:  framerate
    /// Value:  framerate-value
    /// Usage Level:  media
    /// Charset Dependent:  no
    /// 
    /// Syntax:
    /// framerate-value = non-zero-int-or-real
    /// 
    /// Example:
    /// a=framerate:60
    /// 
    /// This gives the maximum video frame rate in frames/sec.  It is
    /// intended as a recommendation for the encoding of video data.  Decimal
    /// representations of fractional values are allowed.  It is defined only
    /// for video media.
    pub framerate: Option<u16>,
    /// Name:  quality
    /// Value:  quality-value
    /// Usage Level:  media
    /// Charset Dependent:  no
    /// 
    /// Syntax:
    /// quality-value = zero-based-integer
    /// 
    /// Example:
    /// a=quality:10
    /// 
    /// This gives a suggestion for the quality of the encoding as an integer
    /// value.  The intention of the quality attribute for video is to
    /// specify a non-default trade-off between frame-rate and still-image
    /// quality.  For video, the value is in the range 0 to 10, with the
    /// following suggested meaning:
    /// 
    /// +----+----------------------------------------+
    /// | 10 | the best still-image quality the       |
    /// |    | compression scheme can give.           |
    /// +----+----------------------------------------+
    /// | 5  | the default behavior given no quality  |
    /// |    | suggestion.                            |
    /// +----+----------------------------------------+
    /// | 0  | the worst still-image quality the      |
    /// |    | codec designer thinks is still usable. |
    /// +----+----------------------------------------+
    pub quality: Option<u8>,
    /// Name:  type
    /// Value:  type-value
    /// Usage Level:  session
    /// Charset Dependent:  no
    /// 
    /// Syntax:
    /// type-value = conference-type
    /// conference-type = broadcast / meeting / moderated / test / H332
    /// broadcast = %s"broadcast"
    /// meeting   = %s"meeting"
    /// moderated = %s"moderated"
    /// test      = %s"test"
    /// H332      = %s"H332"
    /// ; NOTE: These names are case-sensitive.
    /// 
    /// Example:
    /// a=type:moderated
    pub kind: Option<Kind>,
    /// Name:  recvonly
    /// Value:
    /// Usage Level:  session, media
    /// Charset Dependent:  no
    /// 
    /// Example:
    /// a=recvonly
    /// 
    /// This specifies that the tools should be started in receive-only mode
    /// where applicable.  Note that receive-only mode applies to the media
    /// only, not to any associated control protocol.  An RTP-based system in
    /// receive-only mode MUST still send RTCP packets as described in
    /// [RFC3550](https://datatracker.ietf.org/doc/html/rfc3550#section-6).
    pub recvonly: bool,
    /// Name:  sendonly
    /// Value:
    /// Usage Level:  session, media
    /// Charset Dependent:  no
    /// 
    /// Example:
    /// a=sendonly
    /// 
    /// This specifies that the tools should be started in send-only mode.
    /// An example may be where a different unicast address is to be used for
    /// a traffic destination than for a traffic source.  In such a case, two
    /// media descriptions may be used, one in send-only mode and one in
    /// receive-vonly mode.  Note that send-only mode applies only to the
    /// media, and any associated control protocol (e.g., RTCP) SHOULD still
    /// be received and processed as normal.
    pub sendrecv: bool,
    /// Name:  inactive
    /// Value:
    /// Usage Level:  session, media
    /// Charset Dependent:  no
    /// 
    /// Example:
    /// a=inactive
    /// 
    /// This specifies that the tools should be started in inactive mode.
    /// This is necessary for interactive multimedia conferences where users
    /// can put other users on hold.  No media is sent over an inactive media
    /// stream.  Note that an RTP-based system MUST still send RTCP (if RTCP
    /// is used), even if started in inactive mode.
    pub sendonly: bool,
    /// Name:  inactive
    /// Value:
    /// Usage Level:  session, media
    /// Charset Dependent:  no
    /// 
    /// Example:
    /// a=inactive
    /// 
    /// This specifies that the tools should be started in inactive mode.
    /// This is necessary for interactive multimedia conferences where users
    /// can put other users on hold.  No media is sent over an inactive media
    /// stream.  Note that an RTP-based system MUST still send RTCP (if RTCP
    /// is used), even if started in inactive mode.
    pub inactive: bool,
    /// SDP extmap Attribute
    pub extmap: ExtMap<'a>,
    
    pub mid: Option<Mid>,
    
}

impl<'a> Attributes<'a> {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    /// use std::convert::*;
    ///
    /// let value: RtpValue = RtpValue::try_from("VP8/9000")
    ///     .unwrap();
    /// 
    /// assert_eq!(value.codec, Codec::Vp8);
    /// assert_eq!(value.frequency, Some(9000));
    /// assert_eq!(value.channels, None);
    /// ```
    pub fn handle(&mut self, line: &'a str) -> Result<()> {
        let (k, v) = tuple2_from_split(line, ':', "invalid attributes!")?;
        let key = match Key::try_from(k) {
            Ok(k) => k,
            _ => return Ok(())
        };
        
        Ok(match key {
            Key::Fmtp      => self.fmtp.insert(v)?,
            Key::RtpMap    => self.rtpmap.insert(v)?,
            Key::ExtMap    => self.extmap.insert(v)?,
            Key::Lang      => self.lang = Some(v),
            Key::Charset   => self.charset = Some(v),
            Key::SdpLang   => self.sdplang = Some(v),
            Key::Ptime     => self.ptime = Some(v.parse()?),
            Key::MaxPtime  => self.maxptime = Some(v.parse()?),
            Key::Orient    => self.orient = Some(Orient::try_from(v)?),
            Key::Type      => self.kind = Some(Kind::try_from(v)?),
            Key::Framerate => self.framerate = Some(v.parse()?),
            Key::Quality   => self.quality = Some(v.parse()?),
        })
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
            Self::Fmtp      => "fmtp",
            Self::Lang      => "lang",
            Self::RtpMap    => "rtpmap",
            Self::ExtMap    => "extmap",
            Self::Charset   => "charset",
            Self::SdpLang   => "sdplang",
            Self::Ptime     => "ptime",
            Self::MaxPtime  => "maxptime",
            Self::Orient    => "orient",
            Self::Type      => "type",
            Self::Framerate => "framerate",
            Self::Quality   => "quality",
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
            "fmtp"      => Ok(Self::Fmtp),
            "lang"      => Ok(Self::Lang),
            "rtpmap"    => Ok(Self::RtpMap),
            "extmap"    => Ok(Self::ExtMap),
            "charset"   => Ok(Self::Charset),
            "sdplang"   => Ok(Self::SdpLang),
            "ptime"     => Ok(Self::Ptime),
            "maxptime"  => Ok(Self::MaxPtime),
            "orient"    => Ok(Self::Orient),
            "type"      => Ok(Self::Type),
            "framerate" => Ok(Self::Framerate),
            "quality"   => Ok(Self::Quality),
            _ => Err(anyhow!("invalid sdp attributes keys!"))
        }
    }
}
