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
