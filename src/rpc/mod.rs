mod hooks;
mod server;

pub use self::hooks::{HooksEvent, RpcHooksService};

use self::server::RpcServer;
use crate::{Service, config::Config, statistics::Statistics};

use std::time::{Duration, Instant};

use anyhow::Result;
use axum::{
    Router,
    error_handling::HandleErrorLayer,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use sdk::protos::turn_service_server::TurnServiceServer;
use tokio::net::TcpListener;
use tonic::service::Routes;
use tower::{BoxError, ServiceBuilder, timeout::TimeoutLayer};

#[cfg(feature = "prometheus")]
use axum::{http::header::CONTENT_TYPE, routing::get};

#[cfg(feature = "prometheus")]
async fn metrics(statistics: Statistics) -> Response {
    let mut buf = Vec::with_capacity(4096);

    if statistics.encode_prometheus(&mut buf).is_err() {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    } else {
        (
            [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
            buf,
        )
            .into_response()
    }
}

pub async fn start_server(config: Config, service: Service, statistics: Statistics) -> Result<()> {
    if let Some(api) = &config.api {
        let timeout_duration = Duration::from_secs(api.timeout as u64);
        let mut app: Router = Routes::new(TurnServiceServer::new(RpcServer {
            config: config.clone(),
            uptime: Instant::now(),
            statistics: statistics.clone(),
            service,
        }))
        .into_axum_router();

        #[cfg(feature = "prometheus")]
        {
            app = app.route(
                "/metrics",
                get({
                    let statistics = statistics.clone();
                    move || metrics(statistics.clone())
                }),
            );
        }

        app = app.layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|_: BoxError| async {
                    StatusCode::REQUEST_TIMEOUT
                }))
                .layer(TimeoutLayer::new(timeout_duration)),
        );

        log::info!("api server listening: listen={}", api.listen);

        #[cfg(feature = "ssl")]
        if let Some(ssl) = &api.ssl {
            let tls = axum_server::tls_rustls::RustlsConfig::from_pem_chain_file(
                ssl.certificate_chain.clone(),
                ssl.private_key.clone(),
            )
            .await?;

            axum_server::bind_rustls(api.listen, tls)
                .serve(app.into_make_service())
                .await?;

            return Ok(());
        }

        let listener = TcpListener::bind(api.listen).await?;
        axum::serve(listener, app).await?;
    } else {
        std::future::pending().await
    }

    Ok(())
}
