use super::Codec;
use anyhow::{
    Result,
    ensure
};

use std::{
    convert::TryFrom, 
    fmt
};

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
        Ok(Self {
            codec: Codec::try_from(values[0])?,
            frequency: if let Some(c) = values.get(1) { Some(c.parse()?) } else { None },
            channels: if let Some(c) = values.get(2) { Some(c.parse()?) } else { None }
        })
    }
}
