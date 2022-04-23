use super::util::short_time;
use itertools::Itertools;
use std::{
    convert::TryFrom,
    fmt
};

/// time zone.
#[derive(Debug)]
pub struct TimeZone {
    pub adjustment_time: u64,
    pub offset: f64
}

/// Time Zones ("z=")
/// 
/// z=<adjustment time> <offset> <adjustment time> <offset> ....
/// 
/// To schedule a repeated session that spans a change from daylight
/// saving time to standard time or vice versa, it is necessary to
/// specify offsets from the base time.  This is required because
/// different time zones change time at different times of day, different
/// countries change to or from daylight saving time on different dates,
/// and some countries do not have daylight saving time at all.
/// 
/// Thus, in order to schedule a session that is at the same time winter
/// and summer, it must be possible to specify unambiguously by whose
/// time zone a session is scheduled.  To simplify this task for
/// receivers, we allow the sender to specify the NTP time that a time
/// zone adjustment happens and the offset from the time when the session
/// was first scheduled.  The "z=" field allows the sender to specify a
/// list of these adjustment times and offsets from the base time.
/// 
/// An example might be the following:
/// 
/// z=2882844526 -1h 2898848070 0
/// 
/// This specifies that at time 2882844526, the time base by which the
/// session's repeat times are calculated is shifted back by 1 hour, and
/// that at time 2898848070, the session's original time base is
/// restored.  Adjustments are always relative to the specified start
/// time -- they are not cumulative.  Adjustments apply to all "t=" and
/// "r=" lines in a session description.
/// 
/// If a session is likely to last several years, it is expected that the
/// session announcement will be modified periodically rather than
/// transmit several years' worth of adjustments in one session
/// announcement.
#[rustfmt::skip]
#[derive(Debug)]
pub struct TimeZones(
    pub Vec<TimeZone>
);

impl TimeZones {
    pub fn get_values(&self) -> &Vec<TimeZone> {
        &self.0
    }
}

impl fmt::Display for TimeZone {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::time_zones::*;
    ///
    /// let temp = "2882844526 100".to_string();
    /// let time_zone = TimeZone {
    ///     adjustment_time: 2882844526,
    ///     offset: 100.0
    /// };
    ///
    /// assert_eq!(format!("{}", time_zone), temp);
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.adjustment_time, self.offset)
    }
}

impl<'a> TryFrom<(&'a str, &'a str)> for TimeZone {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::time_zones::*;
    /// use std::convert::*;
    ///
    /// let temp = ("2882844526", "100");
    /// let instance: TimeZone = TimeZone::try_from(temp).unwrap();
    /// 
    /// assert_eq!(instance.adjustment_time, 2882844526);
    /// assert_eq!(instance.offset, 100.0);
    /// ```
    fn try_from(value: (&'a str, &'a str)) -> Result<Self, Self::Error> {
        Ok(Self {
            adjustment_time: value.0.parse()?,
            offset: short_time(value.1)? as f64
        })
    }
}

impl fmt::Display for TimeZones {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::time_zones::*;
    ///
    /// let temp = "2882844526 100 2898848070 0".to_string();
    /// let time_zones: TimeZones = TimeZones(
    ///     vec![
    ///         TimeZone {
    ///             adjustment_time: 2882844526,
    ///             offset: 100.0
    ///         },
    ///         TimeZone {
    ///             adjustment_time: 2898848070,
    ///             offset: 0.0
    ///         }
    ///     ]
    /// );
    /// 
    /// assert_eq!(format!("{}", time_zones), temp);
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, time_zone) in self.0.iter().enumerate() {
            match index == self.0.len() - 1 {
                true => write!(f, "{}", time_zone),
                false => write!(f, "{} ", time_zone)
            }?;
        }
        
        Ok(())
    }
}

impl<'a> TryFrom<&'a str> for TimeZones {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::time_zones::*;
    /// use std::convert::*;
    ///
    /// let temp = "2882844526 100 2898848070 0";
    /// let instance: TimeZones = TimeZones::try_from(temp).unwrap();
    /// 
    /// assert_eq!(instance.get_values().len(), 2);
    /// assert_eq!(instance.get_values()[0].adjustment_time, 2882844526);
    /// assert_eq!(instance.get_values()[0].offset, 100.0);
    /// assert_eq!(instance.get_values()[1].adjustment_time, 2898848070);
    /// assert_eq!(instance.get_values()[1].offset, 0.0);
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let mut values = Vec::with_capacity(5);
        for (a, b) in value.split(' ').tuples() {
            values.push(TimeZone::try_from((a, b))?);
        }

        Ok(Self(values))
    }
}
