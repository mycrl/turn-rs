use std::net::IpAddr;
use super::{
    NetKind,
    AddrKind
};

use anyhow::{
    ensure,
    anyhow
};

use std::convert::{
    TryFrom,
    Into
};

/// Origin
///
/// The "o=" line (origin-field) gives the originator of the session (her
/// username and the address of the user's host) plus a session
/// identifier and version number.
#[derive(Debug)]
pub struct Origin<'a> {
    /// <username>  is the user's login on the originating host, or it is "-"
    /// if the originating host does not support the concept of user IDs.
    /// The <username> MUST NOT contain spaces.
    pub username: Option<&'a str>,
    /// <sess-id>  is a numeric string such that the tuple of <username>,
    /// <sess-id>, <nettype>, <addrtype>, and <unicast-address> forms a
    /// globally unique identifier for the session.  The method of <sess-
    /// id> allocation is up to the creating tool, but a timestamp, in
    /// seconds since January 1, 1900 UTC, is recommended to ensure
    /// uniqueness.
    pub sess_id: &'a str,
    /// <sess-version>  is a version number for this session description.
    /// Its usage is up to the creating tool, so long as <sess-version> is
    /// increased when a modification is made to the session description.
    /// Again, as with <sess-id> it is RECOMMENDED that a timestamp be
    /// used.
    pub sess_version: u8,
    /// <nettype>  is a text string giving the type of network.  Initially,
    /// "IN" is defined to have the meaning "Internet".
    pub nettype: NetKind,
    /// <addrtype>  is a text string giving the type of the address that
    /// follows.  Initially, "IP4" and "IP6" are defined.
    pub addrtype: AddrKind,
    /// <unicast-address>  is an address of the machine from which the
    /// session was created.  For an address type of "IP4", this is either
    /// a fully qualified domain name of the machine or the dotted-decimal
    /// representation of an IP version 4 address of the machine.  For an
    /// address type of "IP6", this is either a fully qualified domain
    /// name of the machine or the address of the machine represented as
    /// specified in Section 4 of [RFC5952](https://tools.ietf.org/html/rfc5952#section-4).  
    /// For both "IP4" and "IP6", the fully qualified domain name is the 
    /// form that SHOULD be given unless this is unavailable, in which case 
    /// a globally unique address MAY be substituted.
    pub unicast_address: IpAddr,
}

impl<'a> Into<String> for Origin<'a> {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::*;
    /// use sdp::origin::*;
    /// use std::convert::*;
    ///
    /// let temp = "- 9216395717180620054 2 IN IP4 127.0.0.1".to_string();
    /// let origin = Origin {
    ///     username: None,
    ///     sess_id: "9216395717180620054",
    ///     sess_version: 2,
    ///     nettype: NetKind::IN,
    ///     addrtype: AddrKind::IP4,
    ///     unicast_address: "127.0.0.1".parse().unwrap()
    /// };
    ///
    /// let instance: String = origin.into();
    /// assert_eq!(instance, temp);
    /// ```
    fn into(self) -> String {
        let nettype: &'static str = self.nettype.into();
        let addrtype: &'static str = self.addrtype.into();
        format!(
            "{} {} {} {} {} {:?}",
            self.username.unwrap_or("-"),
            self.sess_id,
            self.sess_version,
            nettype,
            addrtype,
            self.unicast_address
        )
    }
}

impl<'a> TryFrom<&'a str> for Origin<'a> {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::*;
    /// use sdp::origin::*;
    /// use std::convert::*;
    /// use std::net::IpAddr;
    ///
    /// let addr: IpAddr = "127.0.0.1".parse().unwrap();
    /// let temp = "- 9216395717180620054 2 IN IP4 127.0.0.1";
    /// let instance: Origin = Origin::try_from(temp).unwrap();
    /// 
    /// assert_eq!(instance.username, None);
    /// assert_eq!(instance.sess_id, "9216395717180620054");
    /// assert_eq!(instance.sess_version, 2);
    /// assert_eq!(instance.nettype, NetKind::IN);
    /// assert_eq!(instance.addrtype, AddrKind::IP4);
    /// assert_eq!(instance.unicast_address, addr);
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let values = value.split(' ').collect::<Vec<&str>>();
        ensure!(values.len() == 6, "invalid origin!");
        Ok(Self {
            sess_id: values[1],
            sess_version: values[2].parse()?,
            unicast_address: values[5].parse()?,
            nettype: NetKind::try_from(values[3])?,
            addrtype: AddrKind::try_from(values[4])?,
            username: if values[0] == "-" { None } else { Some(values[0]) },
        })
    }
}
