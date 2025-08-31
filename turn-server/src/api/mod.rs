pub mod events;

use std::{net::SocketAddr, sync::Arc, time::Instant};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Sse, sse::KeepAlive},
    routing::{delete, get},
};

use serde::Deserialize;
use serde_json::json;
use service::{
    Service,
    session::{Identifier, Session, ports::PortAllocator},
};

use tokio::net::TcpListener;

#[cfg(feature = "ssl")]
use axum_server::tls_openssl::OpenSSLConfig;

use crate::{config::Config, observer::Observer, statistics::Statistics};

struct ApiState {
    config: Arc<Config>,
    service: Service<Observer>,
    statistics: Statistics,
    uptime: Instant,
}

#[derive(Deserialize)]
struct QueryParams {
    address: SocketAddr,
    interface: SocketAddr,
}

impl Into<Identifier> for QueryParams {
    fn into(self) -> Identifier {
        Identifier {
            source: self.address,
            interface: self.interface,
        }
    }
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
pub async fn start_server(
    config: Arc<Config>,
    service: Service<Observer>,
    statistics: Statistics,
) -> anyhow::Result<()> {
    let state = Arc::new(ApiState {
        config: config.clone(),
        uptime: Instant::now(),
        statistics,
        service,
    });

    #[allow(unused_mut)]
    let mut app = Router::new()
        .route(
            "/info",
            get(|State(app_state): State<Arc<ApiState>>| async move {
                let session_manager = app_state.service.get_session_manager_ref();
                Json(json!({
                    "software": crate::SOFTWARE,
                    "uptime": app_state.uptime.elapsed().as_secs(),
                    "interfaces": app_state.config.turn.interfaces,
                    "port_capacity": PortAllocator::capacity(),
                    "port_allocated": session_manager.allocated(),
                }))
            }),
        )
        .route(
            "/session",
            get(
                |Query(query): Query<QueryParams>, State(state): State<Arc<ApiState>>| async move {
                    if let Some(Session::Authenticated { username, allocate_port, allocate_channels, permissions, expires, .. }) = state.service.get_session_manager_ref().get_session(&query.into()).get_ref() {
                        Json(json!({
                            "username": username,
                            "permissions": permissions,
                            "channels": allocate_channels,
                            "port": allocate_port,
                            "expires": expires,
                        }))
                        .into_response()
                    } else {
                        StatusCode::NOT_FOUND.into_response()
                    }
                },
            ),
        )
        .route(
            "/session/statistics",
            get(
                |Query(query): Query<QueryParams>, State(state): State<Arc<ApiState>>| async move {
                    let id: Identifier = query.into();
                    if let Some(counts) = state.statistics.get(&id) {
                        Json(json!({
                            "received_bytes": counts.received_bytes,
                            "send_bytes": counts.send_bytes,
                            "received_pkts": counts.received_pkts,
                            "send_pkts": counts.send_pkts,
                            "error_pkts": counts.error_pkts,
                        }))
                        .into_response()
                    } else {
                        StatusCode::NOT_FOUND.into_response()
                    }
                },
            ),
        )
        .route(
            "/session",
            delete(
                |Query(query): Query<QueryParams>, State(state): State<Arc<ApiState>>| async move {
                    if state.service.get_session_manager_ref().refresh(&query.into(), 0) {
                        StatusCode::OK
                    } else {
                        StatusCode::EXPECTATION_FAILED
                    }
                },
            ),
        )
        .route(
            "/events",
            get(|| async move { Sse::new(events::get_event_stream()).keep_alive(KeepAlive::default()) }),
        );

    #[cfg(feature = "prometheus")]
    {
        use crate::statistics::prometheus::generate_metrics;
        use axum::http::header::CONTENT_TYPE;

        let mut metrics_bytes = Vec::with_capacity(4096);

        app = app.route(
            "/metrics",
            get(|| async move {
                metrics_bytes.clear();

                if generate_metrics(&mut metrics_bytes).is_err() {
                    StatusCode::EXPECTATION_FAILED.into_response()
                } else {
                    ([(CONTENT_TYPE, "text/plain")], metrics_bytes).into_response()
                }
            }),
        );
    }

    #[cfg(feature = "ssl")]
    if let Some(ssl) = &config.api.ssl {
        axum_server::bind_openssl(
            config.api.listen,
            OpenSSLConfig::from_pem_chain_file(
                ssl.certificate_chain.clone(),
                ssl.private_key.clone(),
            )?,
        )
        .serve(app.with_state(state).into_make_service())
        .await?;

        return Ok(());
    }

    {
        let listener = TcpListener::bind(config.api.listen).await?;

        log::info!("api server listening={:?}", config.api.listen);

        axum::serve(listener, app.with_state(state)).await?;
    }

    Ok(())
}
