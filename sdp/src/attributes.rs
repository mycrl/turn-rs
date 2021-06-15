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

use super::{
    Encoding
};

pub struct RtpValue {
    pub encoding: Encoding,
    pub clock_rate: Option<u64>,
    pub channels: Option<u8>
}

pub struct Attributes {
    pub ptime: Option<u64>,
    pub maxptime: Option<u64>,
    pub rtpmap: HashMap<u8, RtpValue>
}

impl Attributes {
    pub fn handle(&mut self, line: &str) -> Result<()> {
        let values = line.split(':').collect::<Vec<&str>>();
        ensure!(!values.is_empty(), "invalid attributes!");
        match values[0] {
            "ptime" => values[1].parse()?,
            "maxptime" => values[1].parse()?,
            "rtpmap" => RtpValue::try_from(values[1])?
        }
    }
}

impl<'a> TryFrom<&'a str> for RtpValue {
    type Error = anyhow::Error;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let values = line.split('/').collect::<Vec<&str>>();
        ensure!(!values.is_empty(), "invalid attributes rtpmap!");
        Ok(Self {
            encoding: Encoding::try_from(values[0])?,
            clock_rate: if let Some(c) = values.get(1) { Some(c.parse()?) } else { None },
            channels: if let Some(c) = values.get(2) { Some(c.parse()?) } else { None }
        })
    }
}