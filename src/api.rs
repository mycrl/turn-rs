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
use tokio::net::TcpListener;

use crate::{
    config::Config,
    observer::Observer,
    statistics::Statistics,
    turn::{PortAllocatePools, Service, SessionAddr},
};

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

impl Into<SessionAddr> for QueryParams {
    fn into(self) -> SessionAddr {
        SessionAddr {
            address: self.address,
            interface: self.interface,
        }
    }
}

pub mod events {
    use std::sync::LazyLock;

    use axum::response::sse::Event;
    use serde::Serialize;
    use tokio::sync::broadcast::{Sender, channel};
    use tokio_stream::wrappers::BroadcastStream;

    static CHANNEL: LazyLock<Sender<Event>> = LazyLock::new(|| channel(10).0);

    pub fn get_event_stream() -> BroadcastStream<Event> {
        BroadcastStream::new(CHANNEL.subscribe())
    }

    pub fn send_with_stream<T, F>(event: &str, handle: F)
    where
        F: FnOnce() -> T,
        T: Serialize,
    {
        if CHANNEL.receiver_count() > 0 {
            let _ = CHANNEL.send(Event::default().event(event).json_data(handle()).unwrap());
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
        service,
        statistics,
    });

    #[allow(unused_mut)]
    let mut app = Router::new()
        .route(
            "/info",
            get(|State(app_state): State<Arc<ApiState>>| async move {
                let sessions = app_state.service.get_sessions();
                Json(json!({
                    "software": crate::SOFTWARE,
                    "uptime": app_state.uptime.elapsed().as_secs(),
                    "interfaces": app_state.config.turn.interfaces,
                    "port_capacity": PortAllocatePools::capacity(),
                    "port_allocated": sessions.allocated(),
                }))
            }),
        )
        .route(
            "/session",
            get(
                |Query(query): Query<QueryParams>, State(state): State<Arc<ApiState>>| async move {
                    if let Some(session) = state.service.get_sessions().get_session(&query.into()).get_ref() {
                        Json(json!({
                            "username": session.auth.username,
                            "permissions": session.permissions,
                            "channels": session.allocate.channels,
                            "port": session.allocate.port,
                            "expires": session.expires,
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
                    let addr: SessionAddr = query.into();
                    if let Some(counts) = state.statistics.get(&addr) {
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
                    if state.service.get_sessions().refresh(&query.into(), 0) {
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

    let listener = TcpListener::bind(config.api.bind).await?;

    log::info!("api server listening={:?}", &config.api.bind);

    axum::serve(listener, app.with_state(state)).await?;
    Ok(())
}
