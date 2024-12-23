use core::str;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    config::{Config, Transport},
    observer::Observer,
    statistics::Statistics,
};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get},
    Json, Router,
};

use base64::prelude::*;
use once_cell::sync::Lazy;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder,
};

use serde::Deserialize;
use serde_json::{json, Value};
use tokio::{
    net::TcpListener,
    sync::mpsc::{unbounded_channel, UnboundedSender},
};

use turn::{sessions::Symbol, PortAllocatePools, Service};

static RID: Lazy<String> = Lazy::new(|| random_string(16));

struct AppState {
    config: Arc<Config>,
    service: Service<Observer>,
    statistics: Statistics,
    uptime: Instant,
}

#[derive(Deserialize)]
struct SessionQueryFilter {
    address: SocketAddr,
    interface: SocketAddr,
    transport: Transport,
}

impl Into<Symbol> for SessionQueryFilter {
    fn into(self) -> Symbol {
        Symbol {
            address: self.address,
            interface: self.interface,
            transport: self.transport.into(),
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
    let state = Arc::new(AppState {
        config: config.clone(),
        uptime: Instant::now(),
        service,
        statistics,
    });

    #[allow(unused_mut)]
    let mut app =
        Router::new()
            .route(
                "/info",
                get(|State(app_state): State<Arc<AppState>>| async move {
                    let sessions = app_state.service.get_sessions();
                    Json(json!({
                        "software": concat!(env!("CARGO_PKG_NAME"), ":", env!("CARGO_PKG_VERSION")),
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
                    |Query(query): Query<SessionQueryFilter>,
                     State(state): State<Arc<AppState>>| async move {
                        if let Some(session) = state
                            .service
                            .get_sessions()
                            .get_session(&query.into())
                            .get_ref()
                        {
                            Json(json!({
                                "username": session.auth.username,
                                "password": session.auth.password,
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
                    |Query(query): Query<SessionQueryFilter>,
                     State(state): State<Arc<AppState>>| async move {
                        let symbol: Symbol = query.into();
                        if let Some(counts) = state.statistics.get(&symbol.address) {
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
                    |Query(query): Query<SessionQueryFilter>,
                     State(state): State<Arc<AppState>>| async move {
                        if state
                            .service
                            .get_sessions()
                            .refresh(&query.into(), 0)
                        {
                            StatusCode::OK
                        } else {
                            StatusCode::EXPECTATION_FAILED
                        }
                    },
                ),
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

    let app = app
        .route_layer(middleware::map_response_with_state(
            state.clone(),
            |State(state): State<Arc<AppState>>, mut res: Response| async move {
                let headers = res.headers_mut();
                headers.insert("Nonce", HeaderValue::from_str(&RID).unwrap());
                headers.insert(
                    "Realm",
                    HeaderValue::from_str(&state.config.turn.realm).unwrap(),
                );

                res
            },
        ))
        .with_state(state);

    log::info!("controller server listening={:?}", &config.api.bind);
    axum::serve(TcpListener::bind(config.api.bind).await?, app).await?;

    Ok(())
}

pub struct HooksService {
    client: Arc<Client>,
    tx: UnboundedSender<Value>,
    config: Arc<Config>,
}

impl HooksService {
    pub fn new(config: Arc<Config>) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("Realm", HeaderValue::from_str(&config.turn.realm)?);
        headers.insert("Nonce", HeaderValue::from_str(&RID)?);

        let client = Arc::new(
            ClientBuilder::new()
                .default_headers(headers)
                .timeout(Duration::from_secs(5))
                .build()?,
        );

        // It keeps taking queued events from the queue and sending them to an external
        // hook service.
        let config_ = config.clone();
        let client_ = client.clone();
        let (tx, mut rx) = unbounded_channel::<Value>();
        tokio::spawn(async move {
            if let Some(server) = &config_.api.hooks {
                let uri = format!("{}/events", server);

                while let Some(signal) = rx.recv().await {
                    if let Err(e) = client_.post(&uri).json(&signal).send().await {
                        log::error!("failed to request hooks server, err={}", e);
                    }
                }
            }
        });

        Ok(Self { client, config, tx })
    }

    pub async fn get_password(&self, key: &Symbol, username: &str) -> Option<String> {
        // Match the static authentication information first.
        if let Some(pwd) = self.config.auth.static_credentials.get(username) {
            return Some(pwd.clone());
        }

        // Try again to match the static authentication key.
        if let Some(secret) = &self.config.auth.static_auth_secret {
            // Because (TURN REST api) this RFC does not mandate the format of the username,
            // only suggested values. In principle, the RFC also indicates that the
            // timestamp part of username can be set at will, so the timestamp is not
            // verified here, and the external web service guarantees its security by
            // itself.
            return encode_password(secret, username);
        }

        // There are no matching static entries, get the password from an external hook
        // service.
        if let Some(server) = &self.config.api.hooks {
            if let Ok(res) = self
                .client
                .get(format!(
                    "{}/password?address={}&interface={}&transport={}&username={}",
                    server,
                    key.address,
                    key.interface,
                    match Transport::from(key.transport) {
                        Transport::TCP => "tcp",
                        Transport::UDP => "udp",
                    },
                    username
                ))
                .send()
                .await
            {
                if let Ok(password) = res.text().await {
                    return Some(password);
                }
            }
        }

        None
    }

    // Notifications for all events are all added to the queue, which has the
    // advantage of not blocking the current call, which is useful for scenarios
    // requiring high real-time performance.
    pub fn emit(&self, event: Value) {
        if self.config.api.hooks.is_some() {
            if let Err(e) = self.tx.send(event) {
                log::error!("failed to send event, err={}", e)
            }
        }
    }
}

fn random_string(len: usize) -> String {
    let mut rng = thread_rng();
    std::iter::repeat(())
        .map(|_| rng.sample(Alphanumeric) as char)
        .take(len)
        .collect::<String>()
        .to_lowercase()
}

// https://datatracker.ietf.org/doc/html/draft-uberti-behave-turn-rest-00#section-2.2
fn encode_password(key: &str, username: &str) -> Option<String> {
    Some(
        BASE64_STANDARD.encode(
            stun::util::hmac_sha1(key.as_bytes(), &[username.as_bytes()])
                .ok()?
                .into_bytes()
                .as_slice(),
        ),
    )
}
