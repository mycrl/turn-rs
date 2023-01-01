mod controller;

pub use controller::{
    ExtController,
    Controller,
};

use crate::config::Config;
use std::net::SocketAddr;
use serde::Deserialize;
use axum::{
    extract::Query,
    routing::get,
    Router,
};

#[derive(Debug, Deserialize)]
pub struct GetUserParams {
    addr: SocketAddr,
}

/// start http server
///
/// Create an http server and start it, and you can access the controller
/// instance through the http interface.
///
/// Warn: This http server does not contain
/// any means of authentication, and sensitive information and dangerous
/// operations can be obtained through this service, please do not expose it
/// directly to an unsafe environment.
///
/// # Example
///
/// ```no_run
/// let config = Config::new()
/// let service = Service::new(/* ... */);;
/// let monitor = Monitor::new(/* ... */);
///
/// let router = service.get_router();
/// let ctr = Controller::new(router, config.clone(), monitor);
/// // start(&config, &ctr).await?;
/// ```
pub async fn start(cfg: &Config, ctr: &Controller) -> anyhow::Result<()> {
    let ctr: &'static Controller = unsafe { std::mem::transmute(ctr) };
    let app = Router::new()
        .route("/stats", get(move || async { ctr.get_stats().await }))
        .route("/workers", get(move || async { ctr.get_workers().await }))
        .route("/users", get(move || async { ctr.get_users().await }))
        .route(
            "/user",
            get(move |Query(params): Query<GetUserParams>| async move {
                ctr.get_user(params.addr).await
            }),
        );

    axum::Server::bind(&cfg.controller_bind)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
