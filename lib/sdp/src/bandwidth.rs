use anyhow::{
    ensure,
    anyhow
};

use std::convert::{
    TryFrom,
    Into
};

/// Bandwidth Kind
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

impl Into<String> for Bandwidth {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::bandwidth::*;
    /// use std::convert::*;
    ///
    /// let temp = "AS:128".to_string();
    /// let bandwidth = Bandwidth {
    ///     bwtype: BwKind::AS,
    ///     bandwidth: 128
    /// };
    ///
    /// let instance: String = bandwidth.into();
    /// assert_eq!(instance, temp);
    /// ```
    fn into(self) -> String {
        let bwtype: &'static str = self.bwtype.into();
        format!(
            "{}:{}",
            bwtype,
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
    /// let temp = "AS:128".to_string();
    /// let instance: Bandwidth = Bandwidth::try_from(temp).unwrap();
    /// 
    /// assert_eq!(instance.bwtype, BwKind::AS);
    /// assert_eq!(instance.bandwidth, 128);
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let values = value.split(':').collect::<Vec<&str>>();
        ensure!(values.len() == 2, "invalid band width!");
        Ok(Self {
            bwtype: BwKind::try_from(values[0])?,
            bandwidth: values[1].parse()?,
        })
    }
}

impl Into<&'static str> for BwKind {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::bandwidth::*;
    /// use std::convert::*;
    ///
    /// let kind: &'static str = BwKind::AS.into();
    /// assert_eq!(kind, "AS");
    /// ```
    fn into(self) -> &'static str {
        match self {
            Self::CT => "CT",
            Self::AS => "AS"
        }
    }
}

impl<'a> TryFrom<&'a str> for BwKind {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    //// use sdp::bandwidth::*;
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
