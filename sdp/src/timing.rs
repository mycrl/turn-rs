use anyhow::ensure;
use std::convert::{
    TryFrom,
    Into
};

/// Timing ("t=")
/// 
/// t=<start-time> <stop-time>
/// 
/// The "t=" lines specify the start and stop times for a session.
/// Multiple "t=" lines MAY be used if a session is active at multiple
/// irregularly spaced times; each additional "t=" line specifies an
/// additional period of time for which the session will be active.  If
/// the session is active at regular times, an "r=" line (see below)
/// should be used in addition to, and following, a "t=" line -- in which
/// case the "t=" line specifies the start and stop times of the repeat
/// sequence.
/// 
/// The first and second sub-fields give the start and stop times,
/// respectively, for the session.  These values are the decimal
/// representation of Network Time Protocol (NTP) time values in seconds
/// since 1900.  To convert these values to UNIX time, subtract
/// decimal 2208988800.

/// NTP timestamps are elsewhere represented by 64-bit values, which wrap
/// sometime in the year 2036.  Since SDP uses an arbitrary length
/// decimal representation, this should not cause an issue (SDP
/// timestamps MUST continue counting seconds since 1900, NTP will use
/// the value modulo the 64-bit limit).
/// 
/// If the <stop-time> is set to zero, then the session is not bounded,
/// though it will not become active until after the <start-time>.  If
/// the <start-time> is also zero, the session is regarded as permanent.
#[derive(Debug)]
pub struct Timing {
    pub start: u64,
    pub stop: u64
}

impl Into<String> for Timing {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::timing::*;
    /// use std::convert::*;
    ///
    /// let temp = "0 0".to_string();
    /// let timing = Timing {
    ///     start: 0,
    ///     stop: 0
    /// };
    ///
    /// let instance: String = timing.into();
    /// assert_eq!(instance, temp);
    /// ```
    fn into(self) -> String {
        format!(
            "{} {}",
            self.start,
            self.stop
        )
    }
}

impl<'a> TryFrom<&'a str> for Timing {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::timing::*;
    /// use std::convert::*;
    ///
    /// let temp = "0 0";
    /// let instance: Timing = Timing::try_from(temp).unwrap();
    /// 
    /// assert_eq!(instance.start, 0);
    /// assert_eq!(instance.stop, 0);
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let values = value.split(' ').collect::<Vec<&str>>();
        ensure!(values.len() == 2, "invalid timing!");
        Ok(Self {
            start: values[0].parse::<u64>()?,
            stop: values[1].parse::<u64>()?
        })
    }
}
