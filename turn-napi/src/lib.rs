#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use std::net::SocketAddr;

use async_trait::async_trait;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::ThreadsafeFunction;
use napi::tokio::sync::Mutex;
use napi::{Error, Result, Status};
use turn_rs::{Observer, Processor, Service, StunClass};

struct TurnObserver {
    get_password: Option<ThreadsafeFunction<(String, String)>>,
}

impl TurnObserver {
    fn new(observer: &Object) -> Result<Self> {
        Ok(Self {
            get_password: observer.get::<_, ThreadsafeFunction<(String, String)>>("get_password")?,
        })
    }
}

#[async_trait]
impl Observer for TurnObserver {
    async fn get_password(&self, addr: &SocketAddr, name: &str) -> Option<String> {
        self.get_password
            .as_ref()?
            .call_async::<Option<String>>(Ok((addr.to_string(), name.to_string())))
            .await
            .ok()?
    }
}

#[napi]
pub struct TurnService(Service);

#[napi]
impl TurnService {
    #[napi(constructor)]
    pub fn new(realm: String, externals: Vec<String>, observer: Object) -> Result<Self> {
        let mut externals_ = Vec::with_capacity(externals.len());
        for item in externals {
            externals_.push(string_into_addr(item)?);
        }

        Ok(Self(Service::new(
            realm,
            externals_,
            TurnObserver::new(&observer)?,
        )))
    }

    #[napi]
    pub fn get_processor(&self, interface: String, external: String) -> Result<TurnProcessor> {
        Ok(TurnProcessor(Mutex::new(self.0.get_processor(
            string_into_addr(interface)?,
            string_into_addr(external)?,
        ))))
    }
}

#[napi]
pub enum StunKind {
    Msg,
    Channel,
}

impl From<StunClass> for StunKind {
    fn from(value: StunClass) -> Self {
        match value {
            StunClass::Msg => Self::Msg,
            StunClass::Channel => Self::Channel,
        }
    }
}

#[napi]
pub struct Response {
    pub data: Buffer,
    pub kind: StunKind,
    pub relay: Option<String>,
    pub interface: Option<String>,
}

#[napi]
pub struct TurnProcessor(Mutex<Processor>);

#[napi]
impl TurnProcessor {
    #[napi]
    pub async fn process(&self, buf: Buffer, addr: String) -> Result<Option<Response>> {
        Ok(self
            .0
            .lock()
            .await
            .process(&buf, string_into_addr(addr)?)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?
            .map(|ret| Response {
                data: Buffer::from(ret.data),
                kind: StunKind::from(ret.kind),
                relay: ret.relay.map(|item| item.to_string()),
                interface: ret.interface.map(|item| item.to_string()),
            }))
    }
}

#[inline]
fn string_into_addr(input: String) -> Result<SocketAddr> {
    input
        .parse()
        .map_err(|_| Error::from_status(Status::InvalidArg))
}
