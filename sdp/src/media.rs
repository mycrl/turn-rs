use anyhow::{
    ensure,
    anyhow
};

use std::{
    convert::TryFrom,
    fmt
};

/// media type.
/// 
/// <media> is the media type.  Currently defined media are "audio",
/// "video", "text", "application", and "message"
#[derive(Debug, PartialEq, Eq)]
pub enum MediaKind {
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

/// media port.
/// 
/// <port> is the transport port to which the media stream is sent.  The
/// meaning of the transport port depends on the network being used as
/// specified in the relevant "c=" field, and on the transport
/// protocol defined in the <proto> sub-field of the media field.
/// Other ports used by the media application (such as the RTP Control
/// Protocol (RTCP) port [19](https://datatracker.ietf.org/doc/html/rfc4566#ref-19)) 
/// MAY be derived algorithmically from the base media port or MAY be 
/// specified in a separate attribute.
/// 
/// If non-contiguous ports are used or if they don't follow the
/// parity rule of even RTP ports and odd RTCP ports, the "a=rtcp:"
/// attribute MUST be used.  Applications that are requested to send
/// media to a <port> that is odd and where the "a=rtcp:" is present
/// MUST NOT subtract 1 from the RTP port: that is, they MUST send the
/// RTP to the port indicated in <port> and send the RTCP to the port
/// indicated in the "a=rtcp" attribute.
/// 
/// For applications where hierarchically encoded streams are being
/// sent to a unicast address, it may be necessary to specify multiple
/// transport ports.  This is done using a similar notation to that
/// used for IP multicast addresses in the "c=" field:
/// 
/// m=<media> <port>/<number of ports> <proto> <fmt> ...
/// 
/// In such a case, the ports used depend on the transport protocol.
/// For RTP, the default is that only the even-numbered ports are used
/// for data with the corresponding one-higher odd ports used for the
/// RTCP belonging to the RTP session, and the <number of ports>
/// denoting the number of RTP sessions.  For example:
/// 
/// m=video 49170/2 RTP/AVP 31
/// 
/// would specify that ports 49170 and 49171 form one RTP/RTCP pair
/// and 49172 and 49173 form the second RTP/RTCP pair.  RTP/AVP is the
/// transport protocol and 31 is the format (see below).  If non-
/// contiguous ports are required, they must be signalled using a
/// separate attribute.
/// 
/// If multiple addresses are specified in the "c=" field and multiple
/// ports are specified in the "m=" field, a one-to-one mapping from
/// port to the corresponding address is implied.  For example:
/// 
/// c=IN IP4 224.2.1.1/127/2
/// m=video 49170/2 RTP/AVP 31
/// 
/// would imply that address 224.2.1.1 is used with ports 49170 and
/// 49171, and address 224.2.1.2 is used with ports 49172 and 49173.
/// 
/// The semantics of multiple "m=" lines using the same transport
/// address are undefined.  This implies that, unlike limited past
/// practice, there is no implicit grouping defined by such means and
/// an explicit grouping framework should instead be used to express 
/// the intended semantics.
#[derive(Debug)]
pub struct Port {
    pub num: u16,
    pub count: Option<u8>
}

/// Media Descriptions ("m=")
///
/// m=<media> <port> <proto> <fmt> ...
///
/// A session description may contain a number of media descriptions.
/// Each media description starts with an "m=" field and is terminated by
/// either the next "m=" field or by the end of the session description.
/// A media field has several sub-fields:
#[derive(Debug)]
pub struct Media {
    pub mediatype: MediaKind,
    pub port: Port,
    pub protos: Vec<Proto>,
    /// <fmt> is a media format description.  The fourth and any subsequent
    /// sub-fields describe the format of the media.  The interpretation
    /// of the media format depends on the value of the <proto> sub-field.
    /// 
    /// If the <proto> sub-field is "RTP/AVP" or "RTP/SAVP" the <fmt>
    /// sub-fields contain RTP payload type numbers.  When a list of
    /// payload type numbers is given, this implies that all of these
    /// payload formats MAY be used in the session, but the first of these
    /// formats SHOULD be used as the default format for the session.  For
    /// dynamic payload type assignments the "a=rtpmap:" attribute SHOULD 
    //// be used to map from an RTP payload type number to a media encoding 
    /// name that identifies the payload format.  The "a=fmtp:"  attribute 
    /// MAY be used to specify format parameters.
    /// 
    /// If the <proto> sub-field is "udp" the <fmt> sub-fields MUST
    /// reference a media type describing the format under the "audio",
    /// "video", "text", "application", or "message" top-level media
    /// types.  The media type registration SHOULD define the packet
    /// format for use with UDP transport.
    /// 
    /// For media using other transport protocols, the <fmt> field is
    /// protocol specific.  Rules for interpretation of the <fmt> sub-
    /// field MUST be defined when registering new protocols.
    pub fmts: Vec<u8>
}

impl fmt::Display for Media {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::media::*;
    ///
    /// let media = Media {
    ///     mediatype: MediaKind::Video,
    ///     port: Port {
    ///         num: 9,
    ///         count: Some(2)
    ///     },
    ///     protos: vec![
    ///         Proto::Udp,
    ///         Proto::Tls,
    ///         Proto::Avp,
    ///         Proto::Savp
    ///     ],
    ///     fmts: vec![
    ///         96, 97, 98, 99, 100, 101,
    ///         102, 121, 127, 120, 125
    ///     ]
    /// };
    ///
    /// assert_eq!(
    ///     format!("{}", media), 
    ///     "video 9/2 UDP/TLS/AVP/SAVP 96 97 98 99 100 101 102 121 127 120 125"
    /// );
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, 
            "{} {}", 
            self.mediatype, 
            self.port
        )?;
        
        if !self.protos.is_empty() {
            write!(f, " ")?;
        }
        
        for (i, p) in self.protos.iter().enumerate() {
            match i == self.protos.len() - 1 {
                true => write!(f, "{}", p)?,
                false => write!(f, "{}/", p)?
            }
        }

        if !self.fmts.is_empty() {
            write!(f, " ")?;
        }

        for (i, x) in self.fmts.iter().enumerate() {
            match i == self.fmts.len() - 1 {
                true => write!(f, "{}", x)?,
                false => write!(f, "{} ", x)?
            }
        }

        Ok(())
    }
}

impl<'a> TryFrom<&'a str> for Media {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::media::*;
    /// use std::convert::TryFrom;
    ///
    /// let media: Media = Media::try_from(
    ///     "video 9/2 UDP/TLS/AVP/SAVP 96 97 98 99 100 101 102 121 127 120 125"
    /// ).unwrap();
    ///
    /// assert_eq!(media.mediatype, MediaKind::Video);
    /// assert_eq!(media.port.num, 9);
    /// assert_eq!(media.port.count, Some(2));
    /// 
    /// assert_eq!(media.protos[0], Proto::Udp);
    /// assert_eq!(media.protos[1], Proto::Tls);
    /// assert_eq!(media.protos[2], Proto::Avp);
    /// assert_eq!(media.protos[3], Proto::Savp);
    ///
    /// assert_eq!(
    ///     media.fmts, 
    ///     vec![96, 97, 98, 99, 100, 101, 102, 121, 127, 120, 125]
    /// );
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let values = value.split(' ').collect::<Vec<&str>>();
        ensure!(values.len() >= 3, "invalid media!");

        let mut protos = Vec::with_capacity(5);
        for p in values[2].split('/') {
            protos.push(Proto::try_from(p)?);
        }

        let mut fmts = Vec::with_capacity(15);
        for f in values[3..].iter() {
            fmts.push(f.parse()?);
        }

        Ok(Self {
            mediatype: MediaKind::try_from(values[0])?,
            port: Port::try_from(values[1])?,
            protos,
            fmts
        })
    }
}

impl fmt::Display for Port {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::media::*;
    ///
    /// let port = Port {
    ///     num: 9,
    ///     count: Some(2)
    /// };
    ///
    /// assert_eq!(format!("{}", port), "9/2");
    /// 
    /// let port = Port {
    ///     num: 9,
    ///     count: None
    /// };
    ///
    /// assert_eq!(format!("{}", port), "9");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.num)?;
        if let Some(count) = self.count {
            write!(f, "/{}", count)?;
        }
        
        Ok(())
    }
}

impl<'a> TryFrom<&'a str> for Port {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::media::*;
    /// use std::convert::TryFrom;
    ///
    /// let port: Port = Port::try_from("9").unwrap();
    /// assert_eq!(port.num, 9);
    /// assert_eq!(port.count, None);
    /// 
    /// let port: Port = Port::try_from("9/2").unwrap();
    /// assert_eq!(port.num, 9);
    /// assert_eq!(port.count, Some(2));
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let values = value.split('/').collect::<Vec<&str>>();
        ensure!(!values.is_empty(), "invalid media port!");
        Ok(Self {
            num: values[0].parse()?,
            count: match values.get(1) {
                Some(c) => Some(c.parse()?),
                None => None
            }
        })
    }
}
   

impl fmt::Display for MediaKind {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::media::*;
    ///
    /// assert_eq!(format!("{}", MediaKind::Audio), "audio");
    /// assert_eq!(format!("{}", MediaKind::Video), "video");
    /// assert_eq!(format!("{}", MediaKind::Text), "text");
    /// assert_eq!(format!("{}", MediaKind::Application), "application");
    /// assert_eq!(format!("{}", MediaKind::Message), "message");
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

impl<'a> TryFrom<&'a str> for MediaKind {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::media::*;
    /// use std::convert::TryFrom;
    ///
    /// assert_eq!(MediaKind::try_from("text").unwrap(), MediaKind::Text);
    /// assert_eq!(MediaKind::try_from("audio").unwrap(), MediaKind::Audio);
    /// assert_eq!(MediaKind::try_from("video").unwrap(), MediaKind::Video);
    /// assert_eq!(MediaKind::try_from("message").unwrap(), MediaKind::Message);
    /// assert_eq!(MediaKind::try_from("application").unwrap(), MediaKind::Application);
    /// assert!(MediaKind::try_from("panda").is_err());
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
    /// use sdp::media::*;
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
    /// use sdp::media::*;
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
