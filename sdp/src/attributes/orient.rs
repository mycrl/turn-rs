use anyhow::{
    Result,
    anyhow
};

use std::{
    convert::TryFrom,
    fmt
};

#[derive(Debug, PartialEq, Eq)]
pub enum Orient {
    Portrait,
    Landscape,
    Seascape
}

impl fmt::Display for Orient {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    ///
    /// assert_eq!(format!("{}", Orient::Portrait), "portrait");
    /// assert_eq!(format!("{}", Orient::Landscape), "landscape");
    /// assert_eq!(format!("{}", Orient::Seascape), "seascape");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Portrait =>    "portrait",
            Self::Landscape =>   "landscape",
            Self::Seascape =>    "seascape"
        })
    }
}

impl<'a> TryFrom<&'a str> for Orient {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    /// use std::convert::*;
    ///
    /// assert_eq!(Orient::try_from("portrait").unwrap(), Orient::Portrait);
    /// assert_eq!(Orient::try_from("landscape").unwrap(), Orient::Landscape);
    /// assert_eq!(Orient::try_from("seascape").unwrap(), Orient::Seascape);
    /// assert!(Orient::try_from("av1x").is_err());
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "portrait" =>   Ok(Self::Portrait),
            "landscape" =>  Ok(Self::Landscape),
            "seascape" =>   Ok(Self::Seascape),
            _ => Err(anyhow!("invalid orient!"))
        }
    }
}
