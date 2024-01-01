use std::net::SocketAddr;

use tonic::transport::Server;

pub use crate::proto::hooks_server::Hooks;

pub async fn start_hooks_service(bind: SocketAddr, hooks: impl Hooks) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .add_service(hooks)
        .serve(bind)
        .await?;
    Ok(())
}
