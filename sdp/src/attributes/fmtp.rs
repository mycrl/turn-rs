use crate::util::tuple2_from_split;
use std::collections::HashMap;
use anyhow::Result;

/// This attribute allows parameters that are specific to a
/// particular format to be conveyed in a way that SDP does not
/// have to understand them.  The format must be one of the formats
/// specified for the media.  Format-specific parameters may be any
/// set of parameters required to be conveyed by SDP and given
/// unchanged to the media tool that will use this format.  At most
/// one instance of this attribute is allowed for each format.
/// 
/// It is a media-level attribute, and it is not dependent on
/// charset.
#[derive(Debug)]
pub struct Fmtp<'a>(
    HashMap<(u8, &'a str), &'a str>
);

impl<'a> Fmtp<'a> {
    fn parse_value(&mut self, key: u8, value: &'a str) -> Result<()> {
        let (k, v) = tuple2_from_split(value, '=', "invalid fmtp!")?;
        self.0.insert((key, k), v);
        Ok(())  
    }
    
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    ///
    /// let mut fmtp = Fmtp::default();
    ///
    /// assert!(fmtp.insert("102 level-asymmetry-allowed=1;\
    ///     packetization-mode=1;\
    ///     profile-level-id=42001f").is_ok());
    /// assert!(fmtp.insert("109 apt=108").is_ok());
    /// assert!(fmtp.insert("3 a;b=2").is_err());
    /// ```
    pub fn insert(&mut self, value: &'a str) -> Result<()> {
        let (code, value) = tuple2_from_split(value, ' ', "invalid fmtp!")?;
        let key: u8 = code.parse()?;
        for value in value.split(';') {
            self.parse_value(key, value)?;
        }
        
        Ok(())
    }
}

impl<'a> Default for Fmtp<'a> {
    fn default() -> Self {
        Self(HashMap::with_capacity(50))
    }
}