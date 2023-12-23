use std::sync::Arc;

use tokio::sync::Mutex;
use tonic::{transport::Channel, Request};

use self::proto::{controller_client::ControllerClient, GetReportReply, QueryFilter, Stats};

pub mod proto {
    tonic::include_proto!("turn");
}

pub struct Rpc(Mutex<ControllerClient<Channel>>);

impl Rpc {
    pub async fn new(uri: &str) -> anyhow::Result<Arc<Self>> {
        Ok(Arc::new(Self(Mutex::new(
            ControllerClient::connect(uri.to_string()).await?,
        ))))
    }

    pub async fn get_status(&self) -> anyhow::Result<Stats> {
        Ok(self
            .0
            .lock()
            .await
            .get_stats(Request::new(()))
            .await?
            .into_inner())
    }

    pub async fn get_report(&self, skip: u32, limit: u32) -> anyhow::Result<GetReportReply> {
        Ok(self
            .0
            .lock()
            .await
            .get_report(Request::new(QueryFilter {
                skip: Some(skip),
                limit: Some(limit),
            }))
            .await?
            .into_inner())
    }
}
