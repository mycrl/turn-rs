use anyhow::{
    Result,
    anyhow
};

use std::{
    convert::TryFrom,
    fmt
};

#[derive(Debug, PartialEq, Eq)]
pub enum Codec {
    Vp9,
    Vp8,
    H264,
    H265,
    Av1x,
    Rtx,
    Red,
    Ulpfec,
    Opus
}

impl fmt::Display for Codec {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    ///
    /// assert_eq!(format!("{}", Codec::Vp9), "VP9");
    /// assert_eq!(format!("{}", Codec::Vp8), "VP8");
    /// assert_eq!(format!("{}", Codec::Av1x), "AV1X");
    /// assert_eq!(format!("{}", Codec::H265), "H265");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Vp9 =>    "VP9",
            Self::Vp8 =>    "VP8",
            Self::H264 =>   "H264",
            Self::H265 =>   "H265",
            Self::Av1x =>   "AV1X",
            Self::Rtx =>    "rtx",
            Self::Red =>    "red",
            Self::Ulpfec => "ulpfec",
            Self::Opus  =>  "opus"
        })
    }
}

impl<'a> TryFrom<&'a str> for Codec {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    /// use std::convert::*;
    ///
    /// assert_eq!(Codec::try_from("VP9").unwrap(), Codec::Vp9);
    /// assert_eq!(Codec::try_from("VP8").unwrap(), Codec::Vp8);
    /// assert_eq!(Codec::try_from("H264").unwrap(), Codec::H264);
    /// assert_eq!(Codec::try_from("H265").unwrap(), Codec::H265);
    /// assert_eq!(Codec::try_from("AV1X").unwrap(), Codec::Av1x);
    /// assert!(Codec::try_from("av1x").is_err());
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "VP9" =>    Ok(Self::Vp9),
            "VP8" =>    Ok(Self::Vp8),
            "H264" =>   Ok(Self::H264),
            "H265" =>   Ok(Self::H265),
            "AV1X" =>   Ok(Self::Av1x),
            "rtx" =>    Ok(Self::Rtx),
            "red" =>    Ok(Self::Red),
            "ulpfec" => Ok(Self::Ulpfec),
            "opus" =>   Ok(Self::Opus),
            _ => Err(anyhow!("invalid codec!"))
        }
    }
}
