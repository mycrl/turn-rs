use anyhow::{
    Result,
    ensure,
    anyhow
};

use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt
};

#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    Broadcast,
    Meeting,
    Moderated,
    Test,
    H332
}

impl fmt::Display for Kind {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    ///
    /// assert_eq!(format!("{}", Kind::Broadcast), "broadcast");
    /// assert_eq!(format!("{}", Kind::Meeting), "meeting");
    /// assert_eq!(format!("{}", Kind::Moderated), "moderated");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Broadcast =>  "broadcast",
            Self::Meeting =>    "meeting",
            Self::Moderated =>  "moderated",
            Self::Test =>       "test",
            Self::H332 =>       "H332",
        })
    }
}

impl<'a> TryFrom<&'a str> for Kind {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    /// use std::convert::*;
    ///
    /// assert_eq!(Kind::try_from("broadcast").unwrap(), Kind::Broadcast);
    /// assert_eq!(Kind::try_from("meeting").unwrap(), Kind::Meeting);
    /// assert_eq!(Kind::try_from("moderated").unwrap(), Kind::Moderated);
    /// assert!(Kind::try_from("av1x").is_err());
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "broadcast" =>  Ok(Self::Broadcast),
            "meeting" =>    Ok(Self::Meeting),
            "moderated" =>  Ok(Self::Moderated),
            "test" =>       Ok(Self::Test),
            "H332" =>       Ok(Self::H332),
            _ => Err(anyhow!("invalid type!"))
        }
    }
}
