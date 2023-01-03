mod controller;

pub use controller::{
    ExtController,
    Controller,
};

use crate::config::Config;
use serde::Deserialize;
use http::Request;
use axum::{
    extract::Query,
    Router,
};

use axum::routing::{
    delete,
    get,
};

use std::{
    net::SocketAddr,
    task::Context,
    task::Poll,
};

use tower::{
    Service,
    Layer,
};

/// Layer that adds high level logs to a Service.
#[derive(Default, Clone)]
struct LogLayer;

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(&self, service: S) -> Self::Service {
        LogService {
            service,
        }
    }
}

/// Middleware that adds high level logs to a Service.
#[derive(Clone)]
pub struct LogService<S> {
    service: S,
}

impl<S, Body> Service<Request<Body>> for LogService<S>
where
    S: Service<Request<Body>>,
    Body: std::fmt::Debug,
{
    type Error = S::Error;
    type Future = S::Future;
    type Response = S::Response;

    /// Returns Poll::Ready(Ok(())) when the service is able to process
    /// requests. If the service is at capacity, then Poll::Pending is
    /// returned and the task is notified when the service becomes ready again.
    /// This function is expected to be called while on a task. Generally, this
    /// can be done with a simple futures::future::poll_fn call.
    ///
    /// If Poll::Ready(Err(_)) is returned, the service is no longer able to
    /// service requests and the caller should discard the service instance.
    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    /// Process the request and return the response asynchronously.
    ///
    /// This function is expected to be callable off task. As such,
    /// implementations should take care to not call poll_ready. Before
    /// dispatching a request, poll_ready must be called and return
    /// Poll::Ready(Ok(())).
    fn call(&mut self, req: Request<Body>) -> Self::Future {
        log::info!("controller server: {:?}", req);
        self.service.call(req)
    }
}

#[derive(Debug, Deserialize)]
pub struct AddrParams {
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
            get(move |Query(params): Query<AddrParams>| async move {
                ctr.get_user(params.addr).await
            }),
        )
        .route(
            "/user",
            delete(move |Query(params): Query<AddrParams>| async move {
                ctr.remove(params.addr).await
            }),
        )
        .layer(LogLayer);

    log::info!("controller server listening: {}", &cfg.controller_bind);
    axum::Server::bind(&cfg.controller_bind)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
