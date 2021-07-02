use anyhow::Result;
use std::{
    convert::TryFrom,
    fmt
};

#[derive(Debug, PartialEq, Eq)]
pub enum Mid {
    Audio,
    Video,
    Ref(u8)
}

impl fmt::Display for Mid {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    ///
    /// assert_eq!(format!("{}", Mid::Video), "video");
    /// assert_eq!(format!("{}", Mid::Audio), "audio");
    /// assert_eq!(format!("{}", Mid::Ref(8)), "8");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Audio =>   "audio".to_string(),
            Self::Video =>   "video".to_string(),
            Self::Ref(n) =>  format!("{}", n)
        })
    }
}

impl<'a> TryFrom<&'a str> for Mid {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    /// use std::convert::*;
    ///
    /// assert_eq!(Mid::try_from("video").unwrap(), Mid::Video);
    /// assert_eq!(Mid::try_from("audio").unwrap(), Mid::Audio);
    /// assert_eq!(Mid::try_from("8").unwrap(), Mid::Ref(8));
    /// assert!(Mid::try_from("a").is_err());
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "video" =>  Ok(Self::Video),
            "audio" =>  Ok(Self::Audio),
            _ =>        Ok(Self::Ref(value.parse()?)),
        }
    }
}
