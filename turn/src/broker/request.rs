use serde::Serialize;
use std::{
    net::SocketAddr,
    convert::Into
};

#[derive(Serialize)]
pub struct Auth {
    pub addr: SocketAddr,
    pub username: String
}

impl Into<Vec<u8>> for Auth {
    fn into(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }
}
