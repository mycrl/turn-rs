use super::util::short_time;
use anyhow::{
    ensure,
    Result
};

use std::{
    convert::TryFrom,
    fmt
};

/// Repeat Times ("r=")
/// 
/// r=<repeat interval> <active duration> <offsets from start-time>
/// 
/// "r=" fields specify repeat times for a session.  For example, if a
/// session is active at 10am on Monday and 11am on Tuesday for one hour
/// each week for three months, then the <start-time> in the
/// corresponding "t=" field would be the NTP representation of 10am on
/// the first Monday, the <repeat interval> would be 1 week, the <active
/// duration> would be 1 hour, and the offsets would be zero and 25
/// hours.  The corresponding "t=" field stop time would be the NTP
/// representation of the end of the last session three months later.  By
/// default, all fields are in seconds, so the "r=" and "t=" fields might
/// be the following:
/// 
/// t=3034423619 3042462419
/// r=604800 3600 0 90000
/// 
/// To make description more compact, times may also be given in units of
/// days, hours, or minutes.  The syntax for these is a number
/// immediately followed by a single case-sensitive character.
/// Fractional units are not allowed -- a smaller unit should be used
/// instead.  The following unit specification characters are allowed:
/// 
/// d - days (86400 seconds)
/// h - hours (3600 seconds)
/// m - minutes (60 seconds)
/// s - seconds (allowed for completeness)
/// 
/// Thus, the above session announcement could also have been written:
/// 
/// r=7d 1h 0 25h
/// 
/// Monthly and yearly repeats cannot be directly specified with a single
/// SDP repeat time; instead, separate "t=" fields should be used to
/// explicitly list the session times.
#[derive(Debug)]
pub struct RepeatTimes {
    pub repeat_interval: f64,
    pub active_duration: f64,
    pub offsets_from_start_time: f64
}

impl fmt::Display for RepeatTimes {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::repeat_times::*;
    ///
    /// let temp = "86400 3600 0 1".to_string();
    /// let timing = RepeatTimes {
    ///     repeat_interval: 86400.0,
    ///     active_duration: 3600.0,
    ///     offsets_from_start_time: 1.0
    /// };
    ///
    /// assert_eq!(format!("{}", timing), temp);
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, 
            "{} {} 0 {}",
            self.repeat_interval,
            self.active_duration,
            self.offsets_from_start_time
        )
    }
}

impl<'a> TryFrom<&'a str> for RepeatTimes {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::repeat_times::*;
    /// use std::convert::*;
    ///
    /// let temp = "1d 1h 0 1s";
    /// let instance: RepeatTimes = RepeatTimes::try_from(temp).unwrap();
    /// 
    /// assert_eq!(instance.repeat_interval, 86400.0);
    /// assert_eq!(instance.active_duration, 3600.0);
    /// assert_eq!(instance.offsets_from_start_time, 1.0);
    ///
    /// let temp = "86400 3600 0 1";
    /// let instance: RepeatTimes = RepeatTimes::try_from(temp).unwrap();
    /// 
    /// assert_eq!(instance.repeat_interval, 86400.0);
    /// assert_eq!(instance.active_duration, 3600.0);
    /// assert_eq!(instance.offsets_from_start_time, 1.0);
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let values = value.split(' ').collect::<Vec<&str>>();
        ensure!(values.len() == 4, "invalid timing!");
        Ok(Self {
            repeat_interval: short_time(values[0])?,
            active_duration: short_time(values[1])?,
            offsets_from_start_time: short_time(values[3])?
        })
    }
}
