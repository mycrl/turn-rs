mod codec;
mod kind;
mod orient;
mod rtp_value;

pub use rtp_value::RtpValue;
pub use orient::Orient;
pub use codec::Codec;
pub use kind::Kind;

use itertools::Itertools;
use anyhow::{
    Result,
    ensure
};

use std::{
    collections::HashMap,
    convert::TryFrom
};

#[derive(Debug, Default)]
pub struct Attributes<'a> {
    pub ptime: Option<u64>,
    pub maxptime: Option<u64>,
    pub rtp_map: HashMap<u8, RtpValue>,
    pub fmtp: HashMap<u8, HashMap<&'a str, &'a str>>,
    pub orient: Option<Orient>,
    pub charset: Option<&'a str>,
    pub sdplang: Option<&'a str>,
    pub lang: Option<&'a str>,
    pub framerate: Option<u16>,
    pub quality: Option<u8>,
    pub kind: Option<Kind>,
    pub recvonly: bool,
    pub sendrecv: bool,
    pub sendonly: bool,
    pub inactive: bool,
    pub extmap: HashMap<u8, &'a str>
}

impl<'a> Attributes<'a> {
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
    pub fn handle(&mut self, line: &'a str) -> Result<()> {
        let values = line.split(':').collect::<Vec<&str>>();
        ensure!(!values.is_empty(), "invalid attributes!");
        match values[0] {
            "ptime" => self.handle_ptime(values[1]),
            "maxptime" => self.handle_maxptime(values[1]),
            "rtpmap" => self.handle_rtpmap(values[1]),
            "orient" => self.handle_orient(values[1]),
            "type" => self.handle_kind (values[1]),
            "charset" => self.handle_charset(values[1]),
            "sdplang" => self.handle_sdplang(values[1]),
            "lang" => self.handle_lang(values[1]),
            "framerate" => self.handle_framerate(values[1]),
            "quality" => self.handle_quality(values[1]),
            "fmtp" => self.handle_fmtp(values[1]),
            "extmap" => self.handle_extmap(values[1]),
            _ => Ok(())
        }
    }
    
    fn handle_quality(&mut self, value: &str) -> Result<()> {
        self.quality = Some(value.parse()?);
        Ok(())
    }
    
    fn handle_ptime(&mut self, value: &str) -> Result<()> {
        self.ptime = Some(value.parse()?);
        Ok(())
    }

    fn handle_maxptime(&mut self, value: &str) -> Result<()> {
        self.maxptime = Some(value.parse()?);
        Ok(())
    }
    
    fn handle_orient(&mut self, value: &str) -> Result<()> {
        self.orient = Some(Orient::try_from(value)?);
        Ok(())
    }
    
    fn handle_kind(&mut self, value: &str) -> Result<()> {
        self.kind = Some(Kind::try_from(value)?);
        Ok(())
    }
    
    fn handle_charset(&mut self, value: &'a str) -> Result<()> {
        self.charset = Some(value);
        Ok(())
    }
    
    fn handle_sdplang(&mut self, value: &'a str) -> Result<()> {
        self.sdplang = Some(value);
        Ok(())
    }
    
    fn handle_lang(&mut self, value: &'a str) -> Result<()> {
        self.lang = Some(value);
        Ok(()) 
    }
    
    fn handle_framerate(&mut self, value: &str) -> Result<()> {
        self.framerate = Some(value.parse()?);
        Ok(())
    }
    
    fn handle_rtpmap(&mut self, value: &str) -> Result<()> {
        let values = value.split(' ').collect::<Vec<&str>>();
        ensure!(values.len() == 2, "invalid rtpmap!");
        let rtp = RtpValue::try_from(values[1])?;
        self.rtp_map.insert(values[0].parse()?, rtp);
        Ok(())
    }
    
    fn handle_fmtp(&mut self, value: &'a str) -> Result<()> {
        let values = value.split(' ').collect::<Vec<&str>>();
        ensure!(values.len() == 2, "invalid fmtp!");
        let key: u8 = values[0].parse()?;
        values[1]
            .split(';')
            .map(|x| x.split('=').collect_tuple::<(&'a str, &'a str)>())
            .filter(|x| x.is_some())
            .for_each(|option| {
                let (k, v) = option.unwrap();
                self.fmtp
                    .entry(key)
                    .or_insert_with(|| HashMap::with_capacity(10))
                    .insert(k, v);
            });
        Ok(())
    }

    fn handle_extmap(&mut self, value: &'a str) -> Result<()> {
        let values = value.split(' ').collect::<Vec<&str>>();
        ensure!(values.len() == 2, "invalid extmap!");
        self.extmap.insert(values[0].parse()?, values[1]);
        Ok(())
    }
}
