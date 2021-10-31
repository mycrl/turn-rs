use serde::Serialize;
use std::{
    net::SocketAddr,
    convert::Into
};

/// auth request struct.
#[derive(Serialize)]
pub struct Auth {
    pub addr: SocketAddr,
    pub username: String
}

impl Into<Vec<u8>> for Auth {
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
    fn into(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }
}
