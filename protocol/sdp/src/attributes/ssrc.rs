use crate::util::tuple2_from_split;
use anyhow::{
    Result,
    anyhow,
};

use std::{
    collections::HashMap,
    convert::TryFrom, 
    fmt
};

#[derive(Debug)]
pub struct MsId<'a> {
    pub id: &'a str,
    pub appdata: &'a str,
}

impl<'a> fmt::Display for MsId<'a> {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    ///
    /// assert_eq!(format!("{}", MsId {
    ///     id: "6x9ZxQZqpo19FRr3Q0xsWC2JJ1lVsk2JE0sG",
    ///     appdata: "43d2eec3-7116-4b29-ad33-466c9358bfb3",
    /// }), "6x9ZxQZqpo19FRr3Q0xsWC2JJ1lVsk2JE0sG 43d2eec3-7116-4b29-ad33-466c9358bfb3");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.id, self.appdata)?;
        Ok(())
    }
}

impl<'a> TryFrom<&'a str> for MsId<'a> {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    /// use std::convert::*;
    ///
    /// let value: MsId = MsId::try_from("6x9ZxQZqpo19FRr3Q0xsWC2JJ1lVsk2JE0sG 43d2eec3-7116-4b29-ad33-466c9358bfb3").unwrap();
    /// assert_eq!(value.id, "6x9ZxQZqpo19FRr3Q0xsWC2JJ1lVsk2JE0sG");
    /// assert_eq!(value.appdata, "43d2eec3-7116-4b29-ad33-466c9358bfb3");
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (k, v) = tuple2_from_split(value, ' ', "invalid msid!")?;
        Ok(Self {
            id: k,
            appdata: v,
        })
    }
}

#[derive(Debug)]
pub enum SsrcAttr<'a> {
    Cname(&'a str),
    PreviousSsrc(u32),
    MsId(MsId<'a>),
    MsLabel(&'a str),
    Label(&'a str),
}

impl<'a> fmt::Display for SsrcAttr<'a> {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    ///
    /// assert_eq!(format!("{}", SsrcAttr::Cname("v1SBHP7c76XqYcWx")), "cname:v1SBHP7c76XqYcWx");
    /// assert_eq!(format!("{}", SsrcAttr::MsLabel("6x9ZxQZqpo19FRr3Q0xsWC2JJ1lVsk2JE0sG")), "mslabel:6x9ZxQZqpo19FRr3Q0xsWC2JJ1lVsk2JE0sG");
    /// assert_eq!(format!("{}", SsrcAttr::Label("43d2eec3-7116-4b29-ad33-466c9358bfb3")), "label:43d2eec3-7116-4b29-ad33-466c9358bfb3");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(match self {
            Self::PreviousSsrc(v) =>    write!(f, "{}:{}", "previous-ssrc", v),
            Self::Cname(v) =>           write!(f, "{}:{}", "cname", v),
            Self::MsId(v) =>            write!(f, "{}:{}", "msid", v),
            Self::MsLabel(v) =>         write!(f, "{}:{}", "mslabel", v),
            Self::Label(v) =>           write!(f, "{}:{}", "label", v),
        }?)
    }
}

impl<'a> TryFrom<&'a str> for SsrcAttr<'a> {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    /// use std::convert::*;
    ///
    /// let value: SsrcAttr = SsrcAttr::try_from("cname:v1SBHP7c76XqYcWx").unwrap();
    /// if let SsrcAttr::Cname(c) = value {
    ///     assert_eq!(c, "v1SBHP7c76XqYcWx");
    /// }
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (k, v) = tuple2_from_split(value, ':', "invalid ssrc attr!")?;
        match k {
            "cname" =>          Ok(Self::Cname(v)),
            "mslabel" =>        Ok(Self::MsLabel(v)),
            "label" =>          Ok(Self::Label(v)),
            "msid" =>           Ok(Self::MsId(MsId::try_from(v)?)),
            "previous-ssrc" =>  Ok(Self::PreviousSsrc(v.parse()?)),
            _ =>                Err(anyhow!("invalid ssrc attr!")),
        }
    }
}

#[derive(Debug)]
pub struct Ssrc<'a>(
    HashMap<u32, SsrcAttr<'a>>
);

impl<'a> Ssrc<'a> {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::attributes::*;
    /// use std::convert::*;
    ///
    /// let mut ssrc = Ssrc::default();
    /// 
    /// assert!(ssrc.insert("1175220440 cname:v1SBHP7c76XqYcWx").is_ok());
    /// assert!(ssrc.insert("1175220440 mslabel:6x9ZxQZqpo19FRr3Q0xsWC2JJ1lVsk2JE0sG").is_ok());
    /// assert!(ssrc.insert("1175220440 label:43d2eec3-7116-4b29-ad33-466c9358bfb3").is_ok());
    /// assert!(ssrc.insert("1175220440 name:v1SBHP7c76XqYcWx").is_err());
    /// ```
    pub fn insert(&mut self, value: &'a str) -> Result<()> {
        let (k, v) = tuple2_from_split(value, ' ', "invalid ssrc!")?;
        self.0.insert(k.parse()?, SsrcAttr::try_from(v)?);
        Ok(())
    }
}

impl Default for Ssrc<'_> {
    fn default() -> Self {
        Self(HashMap::with_capacity(10))
    }
}
