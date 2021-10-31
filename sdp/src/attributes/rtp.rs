use crate::util::tuple2_from_split;
use super::Codec;
use anyhow::{
    Result,
    ensure
};

use std::{
    collections::HashMap,
    convert::TryFrom, 
    fmt
};

#[derive(Debug)]
pub struct RtpMap(
    HashMap<u8, RtpValue>
);

/// This attribute maps from an RTP payload type number (as used in an
/// "m=" line) to an encoding name denoting the payload format to be
/// used.  It also provides information on the clock rate and encoding
/// parameters.  Note that the payload type number is indicated in a
/// 7-bit field, limiting the values to inclusively between 0 and 127.
/// 
/// Although an RTP profile can make static assignments of payload type
/// numbers to payload formats, it is more common for that assignment to
/// be done dynamically using "a=rtpmap:" attributes.  As an example of a
/// static payload type, consider u-law PCM encoded single-channel audio
/// sampled at 8 kHz.  This is completely defined in the RTP audio/video
/// profile as payload type 0, so there is no need for an "a=rtpmap:"
/// attribute, and the media for such a stream sent to UDP port 49232 can
/// be specified as:
/// 
/// m=audio 49232 RTP/AVP 0
/// 
/// An example of a dynamic payload type is 16-bit linear encoded stereo
/// audio sampled at 16 kHz.  If we wish to use the dynamic RTP/AVP
/// payload type 98 for this stream, additional information is required
/// o decode it:
/// 
/// m=audio 49232 RTP/AVP 98
/// a=rtpmap:98 L16/16000/2
/// 
/// Up to one "a=rtpmap:" attribute can be defined for each media format
/// specified.  Thus, we might have the following:
/// 
/// m=audio 49230 RTP/AVP 96 97 98
/// a=rtpmap:96 L8/8000
/// a=rtpmap:97 L16/8000
/// a=rtpmap:98 L16/11025/2
/// 
/// RTP profiles that specify the use of dynamic payload types MUST
/// define the set of valid encoding names and/or a means to register
/// encoding names if that profile is to be used with SDP.  The "RTP/AVP"
/// and "RTP/SAVP" profiles use media subtypes for encoding names, under
/// the top-level media type denoted in the "m=" line.  In the example
/// above, the media types are "audio/L8" and "audio/L16".
/// 
/// For audio streams, encoding-params indicates the number of audio
/// channels.  This parameter is OPTIONAL and may be omitted if the
/// number of channels is one, provided that no additional parameters are
/// needed.
/// 
/// For video streams, no encoding parameters are currently specified.
/// 
/// Additional encoding parameters MAY be defined in the future, but
/// codec-specific parameters SHOULD NOT be added.  Parameters added to
/// an "a=rtpmap:" attribute SHOULD only be those required for a session
/// directory to make the choice of appropriate media to participate in a
/// session.  Codec-specific parameters should be added in other
/// attributes (for example, "a=fmtp:").
/// 
/// Note: RTP audio formats typically do not include information about
/// the number of samples per packet.  If a non-default (as defined in
/// the RTP Audio/Video Profile 
/// [RFC3551](https://datatracker.ietf.org/doc/html/rfc3551)) 
/// packetization is required, the "a=ptime:" attribute is used as given 
/// in [Section 6.4](https://datatracker.ietf.org/doc/html/rfc8866#section-6.4).
#[derive(Debug)]
pub struct RtpValue {
    pub codec: Codec,
    pub frequency: Option<u64>,
    pub channels: Option<u8>
}

impl fmt::Display for RtpValue {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    ///
    /// let rtp = RtpValue {
    ///     codec: Codec::Vp9,
    ///     frequency: Some(9000),
    ///     channels: None
    /// };
    ///
    /// assert_eq!(format!("{}", rtp), "VP9/9000");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.codec)?;

        if let Some(frequency) = self.frequency {
            write!(f, "/{}", frequency)?;
        }

        if let Some(channel) = self.channels {
            write!(f, "/{}", channel)?;
        }

        Ok(())
    }
}

impl<'a> TryFrom<&'a str> for RtpValue {
    type Error = anyhow::Error;
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
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let values = value.split('/').collect::<Vec<&str>>();
        ensure!(!values.is_empty(), "invalid attributes rtpmap!");
        ensure!(values[0].len() > 0, "invalid attributes rtpmap!");
        Ok(Self {
            codec: Codec::try_from(values[0])?,
            frequency: if let Some(c) = values.get(1) { Some(c.parse()?) } else { None },
            channels: if let Some(c) = values.get(2) { Some(c.parse()?) } else { None }
        })
    }
}

impl RtpMap {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    /// use std::convert::*;
    ///
    /// let mut rtpmap = RtpMap::default();
    /// 
    /// assert!(rtpmap.insert("107 rtx/90000").is_ok());
    /// assert!(rtpmap.insert("101 rtx/90000/2").is_ok());
    /// assert!(rtpmap.insert("108 H264/90000").is_ok());
    /// assert!(rtpmap.insert("98").is_err());
    /// ```
    pub fn insert(&mut self, value: &str) -> Result<()> {
        let (k, v) = tuple2_from_split(value, ' ', "invalid rtpmap!")?;
        self.0.insert(k.parse()?, RtpValue::try_from(v)?);
        Ok(())
    }
}

impl Default for RtpMap {
    fn default() -> Self {
        Self(HashMap::with_capacity(50))
    }
}