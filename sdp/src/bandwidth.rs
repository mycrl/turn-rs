use super::util::tuple2_from_split;
use anyhow::anyhow;
use std::{
    convert::TryFrom,
    fmt
};

/// Bandwidth Kind
#[derive(Debug, PartialEq, Eq)]
pub enum BwKind {
    CT,
    AS
}

/// Bandwidth
///
/// This OPTIONAL field denotes the proposed bandwidth to be used by the
/// session or media.  The <bwtype> is an alphanumeric modifier giving
/// the meaning of the <bandwidth> figure.  Two values are defined in
/// this specification
#[derive(Debug)]
pub struct Bandwidth {
    /// CT If the bandwidth of a session or media in a session is different
    /// from the bandwidth implicit from the scope, a "b=CT:..." line
    /// SHOULD be supplied for the session giving the proposed upper limit
    /// to the bandwidth used (the "conference total" bandwidth).  The
    /// primary purpose of this is to give an approximate idea as to
    /// whether two or more sessions can coexist simultaneously.  When
    /// using the CT modifier with RTP, if several RTP sessions are part
    /// of the conference, the conference total refers to total bandwidth
    /// of all RTP sessions.
    pub bwtype: BwKind,
    /// AS The bandwidth is interpreted to be application specific (it will
    /// be the application's concept of maximum bandwidth).  Normally,
    /// this will coincide with what is set on the application's "maximum
    /// bandwidth" control if applicable.  For RTP-based applications, AS
    /// gives the RTP "session bandwidth" as defined in Section 6.2 of
    /// [19](https://datatracker.ietf.org/doc/html/rfc4566#ref-19).
    pub bandwidth: usize
}

impl fmt::Display for Bandwidth {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::bandwidth::*;
    ///
    /// let temp = "AS:128".to_string();
    /// let bandwidth = Bandwidth {
    ///     bwtype: BwKind::AS,
    ///     bandwidth: 128
    /// };
    ///
    /// assert_eq!(format!("{}", bandwidth), temp);
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}",
            self.bwtype,
            self.bandwidth
        )
    }
}

impl<'a> TryFrom<&'a str> for Bandwidth {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::bandwidth::*;
    /// use std::convert::*;
    ///
    /// let temp = "AS:128";
    /// let instance: Bandwidth = Bandwidth::try_from(temp).unwrap();
    /// 
    /// assert_eq!(instance.bwtype, BwKind::AS);
    /// assert_eq!(instance.bandwidth, 128);
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (t, w) = tuple2_from_split(value, ':', "invalid band width!")?;
        Ok(Self {
            bwtype: BwKind::try_from(t)?,
            bandwidth: w.parse()?,
        })
    }
}

impl fmt::Display for BwKind {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::bandwidth::*;
    ///
    /// assert_eq!(format!("{}", BwKind::AS), "AS");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::CT => "CT",
            Self::AS => "AS"
        })
    }
}

impl<'a> TryFrom<&'a str> for BwKind {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::bandwidth::*;
    /// use std::convert::*;
    ///
    /// let kind: BwKind = BwKind::try_from("AS").unwrap();
    /// assert_eq!(kind, BwKind::AS);
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "CT" => Ok(Self::CT),
            "AS" => Ok(Self::AS),
            _ => Err(anyhow!("invalid band width type!"))
        }
    }
}
