use serde::Deserialize;
use std::convert::TryFrom;
use anyhow::{
    Result,
    anyhow
};

#[derive(Deserialize)]
pub struct Auth {
    pub password: String,
    pub group: u32,
}

#[derive(Deserialize)]
pub struct Response<T> {
    pub error: Option<String>,
    pub data: Option<T>
}

impl<'a, T: Deserialize<'a>> TryFrom<&'a [u8]> for Response<T> {
    type Error = anyhow::Error;
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(serde_json::from_slice(value)?)
    }
}

impl<T> Response<T> {
    pub fn into_result(self) -> Result<T> {
        match self.error {
            Some(e) => Err(anyhow!(e)),
            None => match self.data {
                None => Err(anyhow!("bad response!")),
                Some(a) => Ok(a)
            }
        }
    }
}
