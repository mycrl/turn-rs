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
pub enum Codec {
    Vp9,
    Vp8,
    H264,
    H265,
    Av1x,
    Rtx,
    Red,
    Ulpfec
}

#[derive(Debug)]
pub struct RtpValue {
    pub codec: Codec,
    pub frequency: Option<u64>,
    pub channels: Option<u8>
}

#[derive(Debug)]
pub struct Attributes {
    pub ptime: Option<u64>,
    pub maxptime: Option<u64>,
    pub rtp_map: HashMap<u8, RtpValue>
}

impl Attributes {
    fn handle_ptime(&mut self, value: &str) -> Result<()> {
        self.ptime = Some(value.parse()?);
        Ok(())
    }

    fn handle_maxptime(&mut self, value: &str) -> Result<()> {
        self.maxptime = Some(value.parse()?);
        Ok(())
    }
    
    fn handle_rtpmap(&mut self, value: &str) -> Result<()> {
        let values = value.split(' ').collect::<Vec<&str>>();
        ensure!(values.len() == 2, "invalid rtpmap!");
        let rtp = RtpValue::try_from(values[1])?;
        self.rtp_map.insert(values[0].parse()?, rtp);
        Ok(())
    }

    pub fn handle(&mut self, line: &str) -> Result<()> {
        let values = line.split(':').collect::<Vec<&str>>();
        ensure!(!values.is_empty(), "invalid attributes!");
        match values[0] {
            "ptime" => self.handle_ptime(values[1]),
            "maxptime" => self.handle_maxptime(values[1]),
            "rtpmap" => self.handle_rtpmap(values[1]),
            _ => Ok(())
        }
    }
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            ptime: None,
            maxptime: None,
            rtp_map: HashMap::with_capacity(30)
        }
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
            Self::Ulpfec => "ulpfec"
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
            _ => Err(anyhow!("invalid codec!"))
        }
    }
}
