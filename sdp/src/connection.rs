use std::net::IpAddr;
use anyhow::ensure;
use super::{
    NetKind,
    AddrKind
};

use std::convert::{
    TryFrom,
    Into
};

/// Connection Information
///
/// The "c=" line (connection-field) contains information necessary to
/// establish a network connection.
#[derive(Debug)]
pub struct Connection {
    /// <nettype>  is a text string giving the type of network.  Initially,
    /// "IN" is defined to have the meaning "Internet".
    pub nettype: NetKind,
    /// <addrtype>  is a text string giving the type of the address that
    /// follows.  Initially, "IP4" and "IP6" are defined.
    pub addrtype: AddrKind,
    /// (<connection-address>) is the connection address.
    /// Additional subfields MAY be added after the connection address
    /// depending on the value of the <addrtype> subfield.
    pub connection_address: IpAddr,
}

impl Into<String> for Connection {
    /// # Unit Test
    ///
    /// ```
    /// use sdp::*;
    /// use sdp::connection::*;
    /// use std::convert::*;
    ///
    /// let temp = "IN IP4 0.0.0.0".to_string();
    /// let connection = Connection {
    ///     nettype: NetKind::IN,
    ///     addrtype: AddrKind::IP4,
    ///     connection_address: "0.0.0.0".parse().unwrap()
    /// };
    ///
    /// let instance: String = connection.into();
    /// assert_eq!(instance, temp);
    /// ```
    fn into(self) -> String {
        let nettype: &'static str = self.nettype.into();
        let addrtype: &'static str = self.addrtype.into();
        format!(
            "{} {} {:?}",
            nettype,
            addrtype,
            self.connection_address
        )
    }
}

impl<'a> TryFrom<&'a str> for Connection {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use sdp::*;
    /// use sdp::connection::*;
    /// use std::convert::*;
    /// use std::net::IpAddr;
    ///
    /// let temp = "IN IP4 0.0.0.0";
    /// let addr: IpAddr = "0.0.0.0".parse().unwrap();
    /// let instance: Connection = Connection::try_from(temp).unwrap();
    /// 
    /// assert_eq!(instance.nettype, NetKind::IN);
    /// assert_eq!(instance.addrtype, AddrKind::IP4);
    /// assert_eq!(instance.connection_address, addr);
    /// ```
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let values = value.split(' ').collect::<Vec<&str>>();
        ensure!(values.len() == 3, "invalid connection information!");
        Ok(Self {
            nettype: NetKind::try_from(values[0])?,
            addrtype: AddrKind::try_from(values[1])?,
            connection_address: values[2].parse()?,
        })
    }
}
