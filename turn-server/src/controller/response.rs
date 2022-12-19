use std::net::SocketAddr;
use serde::{
    Serialize,
    Deserialize,
};

use std::convert::{
    Into,
    From,
    TryFrom,
};

use anyhow::{
    Result,
    anyhow,
};

/// auth response struct.
#[derive(Deserialize)]
pub struct Auth {
    pub secret: String,
}

/// close response struct.
#[derive(Serialize)]
pub struct Close {
    pub addr: String,
}

/// state response struct.
#[derive(Serialize)]
pub struct State {
    pub capacity: usize,
    pub len: usize,
    pub users: Vec<(String, Vec<SocketAddr>)>,
}

#[derive(Serialize)]
pub struct Node {
    pub channels: Vec<u16>,
    pub ports: Vec<u16>,
    pub timer: usize,
    pub lifetime: u64,
}

impl From<turn::Node> for Node {
    /// # Example
    ///
    /// ```no_run
    /// let res_buf = [
    ///     0x7b, 0x22, 0x65, 0x72, 0x72,
    ///     0x6f, 0x72, 0x22, 0x3a, 0x6e,
    ///     0x75, 0x6c, 0x6c, 0x2c, 0x22,
    ///     0x64, 0x61, 0x74, 0x61, 0x22,
    ///     0x3a, 0x7b, 0x22, 0x67, 0x72,
    ///     0x6f, 0x75, 0x70, 0x22, 0x3a,
    ///     0x30, 0x2c, 0x22, 0x70, 0x61,
    ///     0x73, 0x73, 0x77, 0x6f, 0x72,
    ///     0x64, 0x22, 0x3a, 0x22, 0x70,
    ///     0x61, 0x6e, 0xx64, 0x61, 0x22,
    ///     0x7d, 0x7d
    /// ];
    ///
    /// // Response<Auth>::try_from(&res_buf[..]).unwrap()
    /// ```
    fn from(node: turn::Node) -> Self {
        Self {
            timer: node.timer.elapsed().as_millis() as usize,
            channels: node.channels,
            lifetime: node.lifetime,
            ports: node.ports,
        }
    }
}

/// response from nats request.
///
/// data is empty when error is not empty.
#[derive(Deserialize, Serialize)]
pub struct Response<T> {
    pub error: Option<String>,
    pub data: Option<T>,
}

impl<'a, T: Deserialize<'a>> TryFrom<&'a [u8]> for Response<T> {
    type Error = anyhow::Error;

    /// # Example
    ///
    /// ```no_run
    /// let res_buf = [
    ///     0x7b, 0x22, 0x65, 0x72, 0x72,
    ///     0x6f, 0x72, 0x22, 0x3a, 0x6e,
    ///     0x75, 0x6c, 0x6c, 0x2c, 0x22,
    ///     0x64, 0x61, 0x74, 0x61, 0x22,
    ///     0x3a, 0x7b, 0x22, 0x67, 0x72,
    ///     0x6f, 0x75, 0x70, 0x22, 0x3a,
    ///     0x30, 0x2c, 0x22, 0x70, 0x61,
    ///     0x73, 0x73, 0x77, 0x6f, 0x72,
    ///     0x64, 0x22, 0x3a, 0x22, 0x70,
    ///     0x61, 0x6e, 0xx64, 0x61, 0x22,
    ///     0x7d, 0x7d
    /// ];
    ///
    /// // Response<Auth>::try_from(&res_buf[..]).unwrap()
    /// ```
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(serde_json::from_slice(value)?)
    }
}

impl<T: Serialize> Into<Vec<u8>> for Response<T> {
    /// # Example
    ///
    /// ```no_run
    /// let res_buf = [
    ///     0x7b, 0x22, 0x65, 0x72, 0x72,
    ///     0x6f, 0x72, 0x22, 0x3a, 0x6e,
    ///     0x75, 0x6c, 0x6c, 0x2c, 0x22,
    ///     0x64, 0x61, 0x74, 0x61, 0x22,
    ///     0x3a, 0x7b, 0x22, 0x67, 0x72,
    ///     0x6f, 0x75, 0x70, 0x22, 0x3a,
    ///     0x30, 0x2c, 0x22, 0x70, 0x61,
    ///     0x73, 0x73, 0x77, 0x6f, 0x72,
    ///     0x64, 0x22, 0x3a, 0x22, 0x70,
    ///     0x61, 0x6e, 0xx64, 0x61, 0x22,
    ///     0x7d, 0x7d
    /// ];
    ///
    /// // Response<Auth>::try_from(&res_buf[..]).unwrap()
    /// ```
    fn into(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }
}

impl<T> Response<T> {
    /// into Result from Response.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let res_buf = [
    ///     0x7b, 0x22, 0x65, 0x72, 0x72,
    ///     0x6f, 0x72, 0x22, 0x3a, 0x6e,
    ///     0x75, 0x6c, 0x6c, 0x2c, 0x22,
    ///     0x64, 0x61, 0x74, 0x61, 0x22,
    ///     0x3a, 0x7b, 0x22, 0x67, 0x72,
    ///     0x6f, 0x75, 0x70, 0x22, 0x3a,
    ///     0x30, 0x2c, 0x22, 0x70, 0x61,
    ///     0x73, 0x73, 0x77, 0x6f, 0x72,
    ///     0x64, 0x22, 0x3a, 0x22, 0x70,
    ///     0x61, 0x6e, 0xx64, 0x61, 0x22,
    ///     0x7d, 0x7d
    /// ];
    ///
    /// let res = Response<Auth>::try_from(&res_buf[..])
    ///     .unwrap()
    ///     .into_result()
    ///     .unwrap();
    /// // res.password
    /// ```
    pub fn into_result(self) -> Result<T> {
        match self.error {
            Some(e) => Err(anyhow!(e)),
            None => match self.data {
                None => Err(anyhow!("bad response!")),
                Some(a) => Ok(a),
            },
        }
    }

    /// builde result from params.
    ///
    /// # Example
    ///
    /// ```no_run
    /// Response<()>::from(Some("failed!".to_string()), None);
    /// ```
    pub fn from(error: Option<String>, data: Option<T>) -> Self
    where
        T: Serialize,
    {
        Self {
            error,
            data,
        }
    }
}
