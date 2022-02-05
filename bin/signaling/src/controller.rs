use std::sync::Arc;
use async_nats::{
    Connection,
    connect
};

use anyhow::{
    Result,
    anyhow
};

use std::convert::{
    TryFrom,
    Into
};

use serde::{
    Serialize,
    Deserialize
};

/// auth request struct.
#[derive(Serialize)]
pub struct Request {
    pub realm: String,
    pub uid: String,
    pub token: String
}

impl Into<Vec<u8>> for Request {
    /// uncheck input serialization.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    /// 
    /// let buf = Into::<Vec<u8>>::into(Request {
    ///     realm: "localhost".to_string(),
    ///     token: "token".to_string(),
    ///     uid: "test".to_string(),
    /// });
    /// 
    /// let s: [u8; 50] = [
    ///     123, 34, 114, 101, 97, 108, 109, 34, 
    ///     58, 34, 108, 111, 99, 97, 108, 104, 
    ///     111, 115, 116, 34, 44, 34, 117, 105, 
    ///     100, 34, 58, 34, 116, 101, 115, 116, 
    ///     34, 44, 34, 116, 111, 107, 101, 110, 
    ///     34, 58, 34, 116, 111, 107, 101, 110, 
    ///     34, 125
    /// ];
    ///
    /// assert_eq!(&buf[..], &s);
    /// ```
    fn into(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }
}

/// response from nats request.
///
/// data is empty when error is not empty.
#[derive(Deserialize)]
pub struct Response {
    pub error: Option<String>
}

impl TryFrom<&[u8]> for Response {
    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    /// 
    /// let res_buf = [
    ///     0x7b, 0x22, 0x65, 0x72, 0x72,
    ///     0x6f, 0x72, 0x22, 0x3a, 0x22,
    ///     0x65, 0x72, 0x72, 0x6f, 0x72,
    ///     0x22, 0x7d
    /// ];
    ///
    /// let res = Response::try_from(&res_buf[..]).unwrap();
    /// assert_eq!(res.error, Some("error".to_string()));
    /// ```
    type Error = anyhow::Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(serde_json::from_slice(value)?)
    }
}

impl Response {
    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    /// 
    /// let res_buf = [
    ///     0x7b, 0x22, 0x65, 0x72, 0x72,
    ///     0x6f, 0x72, 0x22, 0x3a, 0x22,
    ///     0x65, 0x72, 0x72, 0x6f, 0x72,
    ///     0x22, 0x7d
    /// ];
    ///
    /// let res = Response::try_from(&res_buf[..])
    ///     .unwrap()
    ///     .into_result();
    /// assert!(res.is_err());
    /// ```
    pub fn into_result(self) -> Result<()> {
        match self.error {
            Some(e) => Err(anyhow!(e)),
            None => Ok(())
        }
    }
}

/// signaling controller.
pub struct Controller {
    conn: Connection,
    realm: String,
}

impl Controller {
    /// # Example
    ///
    /// ```ignore
    /// use signaling::*;
    ///
    /// let ctrl = Controller::new(
    ///     "localhost:4222", 
    ///     "localhost"
    /// ).await?;
    /// ```
    pub async fn new(uri: &str, realm: &str) -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            conn: connect(uri).await?,
            realm: realm.to_string(),
        }))
    }

    /// Hand over the token to the control center for 
    /// authentication verification.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use signaling::*;
    ///
    /// let ctrl = Controller::new(
    ///     "localhost:4222", 
    ///     "localhost"
    /// ).await?;
    /// 
    /// // ctrl.auth("test", "token").is_ok()
    /// ```
    pub async fn auth(&self, from: &str,token: &str) -> Result<()> {
        let message = self
            .conn
            .request(
                "signaling.auth", 
                Into::<Vec<u8>>::into(Request { 
                    realm: self.realm.clone(),
                    token: token.to_string(),
                    uid: from.to_string(),
                })
            ).await?;
        Response
            ::try_from(message.data.as_slice())?
            .into_result()
    }
}
