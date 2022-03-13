use std::net::SocketAddr;
use anyhow::Result;
use serde::{
    Serialize,
    Deserialize
};

use std::convert::{
    Into,
    TryFrom,
};

/// auth request struct.
#[derive(Serialize)]
pub struct Auth {
    pub realm: String,
    pub addr: SocketAddr,
    pub username: String
}

impl Into<Vec<u8>> for Auth {
    /// # Example
    ///
    /// ```no_run
    /// Into::<Auth>::into(Auth {
    ///     addr: "127.0.0.1:8080".parse().unwrap(),
    ///     username: "panda".to_string()
    /// })
    /// ```
    fn into(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }
}

#[derive(Deserialize)]
pub struct Close {
    pub username: String
}

impl TryFrom<&[u8]> for Close {
    type Error = anyhow::Error;
    /// uncheck input serialization.
    ///
    /// # Example
    ///
    /// ```no_run
    /// Into::<Auth>::into(Auth {
    ///     addr: "127.0.0.1:8080".parse().unwrap(),
    ///     username: "panda".to_string()
    /// })
    /// ```
    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        Ok(serde_json::from_slice(buf)?)
    }
}

/// get node request struct.
#[derive(Deserialize)]
pub struct Node {
    pub username: String
}
