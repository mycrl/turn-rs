use crate::util::tuple2_from_split;
use std::collections::HashMap;
use anyhow::Result;

/// attribute name (as it will appear in SDP): extmap
/// 
/// long-form attribute name in English: generic header extension map
/// definition
/// 
/// type of attribute (session level, media level, or both): both
/// 
/// whether the attribute value is subject to the charset attribute:
/// not subject to the charset attribute
/// 
/// a one-paragraph explanation of the purpose of the attribute: This
/// attribute defines the mapping from the extension numbers used in
/// packet headers into extension names as documented in
/// specifications and appropriately registered.
#[derive(Debug, Default)]
pub struct ExtMap<'a>(
    HashMap<u8, &'a str>
);

impl<'a> ExtMap<'a> {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    ///
    /// let mut extmap = ExtMap::default();
    ///
    /// assert!(extmap.insert("1 urn:ietf:params:rtp-hdrext:toffset").is_ok());
    /// assert!(extmap.insert("2 http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time").is_ok());
    /// assert!(extmap.insert("3 urn:3gpp:video-orientation").is_ok());
    /// assert!(extmap.insert("4").is_err());
    /// assert!(extmap.insert("4 name panda").is_err());
    /// ```
    pub fn insert(&mut self, value: &'a str) -> Result<()> {
        let (k, v) = tuple2_from_split(value, ' ', "invalid extmap!")?;
        self.0.insert(k.parse()?, v);
        Ok(())
    }
}