mod controller;
mod hooks;

use std::sync::Arc;

use crate::config::Config;

pub use self::{controller::ControllerService, hooks::Hooks};

use proto::controller_server::ControllerServer;
use tonic::transport::Server;

#[rustfmt::skip]
pub static SOFTWARE: &str = concat!(
    env!("CARGO_PKG_NAME"), 
    ":", 
    env!("CARGO_PKG_VERSION")
);

pub mod proto {
    tonic::include_proto!("turn");
}

/// Create a hook instance, hooks are used to communicate with the outside, such
/// as getting passwords and push events, this is to allow notification of
/// external turn server actions and dynamic authentication of the user's
/// identity.
pub async fn create_hooks(cfg: Arc<Config>) -> anyhow::Result<Hooks> {
    Hooks::new(cfg).await
}

/// start rpc server
///
/// Create an rpc server and start it, and you can access the controller
/// instance through the rpc interface.
///
/// Warn: This rpc server does not contain any means of authentication, and
/// sensitive information and dangerous operations can be obtained through this
/// service, please do not expose it directly to an unsafe environment.
pub async fn start_controller_service(cfg: &Config, ctr: ControllerService) -> anyhow::Result<()> {
    Server::builder()
        .add_service(ControllerServer::new(ctr))
        .serve(cfg.controller.bind)
        .await?;
    Ok(())
}
