use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;
use tonic::{transport::Channel, Request};

use self::proto::{
    controller_client::ControllerClient, AddrParams, GetSessionReply, GetUsersReply, QueryFilter,
    Stats,
};

pub mod proto {
    tonic::include_proto!("turn");
}

pub struct Rpc(Mutex<ControllerClient<Channel>>);

impl Rpc {
    pub async fn new(uri: &str) -> Result<Arc<Self>> {
        Ok(Arc::new(Self(Mutex::new(
            ControllerClient::connect(uri.to_string()).await?,
        ))))
    }

    pub async fn get_status(&self) -> Result<Stats> {
        Ok(self
            .0
            .lock()
            .await
            .get_stats(Request::new(()))
            .await?
            .into_inner())
    }

    pub async fn get_users(&self, skip: u32, limit: u32) -> Result<GetUsersReply> {
        Ok(self
            .0
            .lock()
            .await
            .get_users(Request::new(QueryFilter {
                skip: Some(skip),
                limit: Some(limit),
            }))
            .await?
            .into_inner())
    }

    pub async fn get_session(&self, addr: String) -> Result<GetSessionReply> {
        Ok(self
            .0
            .lock()
            .await
            .get_session(Request::new(AddrParams { addr }))
            .await?
            .into_inner())
    }
}
