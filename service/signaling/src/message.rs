use std::convert::TryFrom;
use serde_json::Error;
use anyhow::Result;
use serde::{
    Deserialize, 
    Serialize
};

/// websocket payload message.
#[derive(Serialize, Deserialize, Debug)]
pub struct Payload {
    pub to: Option<String>
}

impl Payload {
    /// get target uid in payload.
    /// 
    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    ///
    /// let data = r#"
    /// {
    ///     "to": "test",
    ///     "name": "John Doe",
    ///     "age": 43,
    ///     "phones": [
    ///         "+44 1234567",
    ///         "+44 2345678"
    ///     ]
    /// }"#;
    /// 
    /// assert_eq!(Payload::get_to(&data).unwrap(), Some("test".to_string()));
    /// ```
    pub fn get_to(value: &str) -> Result<Option<String>, Error> {
        match serde_json::from_str::<Self>(value) {
            Ok(p) => Ok(p.to),
            Err(e) => Err(e)
        }
    }
}

impl TryFrom<&str> for Payload {
    type Error = anyhow::Error;
    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    ///
    /// let data = r#"
    /// {
    ///     "to": "test",
    ///     "name": "John Doe",
    ///     "age": 43,
    ///     "phones": [
    ///         "+44 1234567",
    ///         "+44 2345678"
    ///     ]
    /// }"#;
    ///
    /// let payload = Payload {
    ///     to: Some("test".to_string())
    /// };
    /// 
    /// let p = Payload::try_from(data).unwrap();
    /// assert_eq!(p.to, payload.to);
    /// ```
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(serde_json::from_str::<Self>(value)?)
    }
}
