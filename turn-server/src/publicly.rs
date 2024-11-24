use core::str;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{config::Config, statistics::Statistics};

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

use turn::Service;

static RID: Lazy<String> = Lazy::new(|| random_string(16));

struct AppState {
    config: Arc<Config>,
    service: Service,
    statistics: Statistics,
    uptime: Instant,
}

#[derive(Deserialize)]
struct QueryFilter {
    addr: Option<SocketAddr>,
    username: Option<String>,
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
    service: Service,
    statistics: Statistics,
) -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        config: config.clone(),
        uptime: Instant::now(),
        service,
        statistics,
    });

    #[allow(unused_mut)]
    let mut app = Router::new()
        .route(
            "/info",
            get(|State(state): State<Arc<AppState>>| async move {
                let router = state.service.get_router();
                Json(json!({
                    "software": concat!(env!("CARGO_PKG_NAME"), ":", env!("CARGO_PKG_VERSION")),
                    "uptime": state.uptime.elapsed().as_secs(),
                    "port_allocated": router.len(),
                    "port_capacity": turn::Router::capacity(),
                    "interfaces": state.config.turn.interfaces,
                }))
            }),
        )
        .route(
            "/session",
            get(
                |Query(query): Query<QueryFilter>, State(state): State<Arc<AppState>>| async move {
                    let addrs = if let Some(username) = query.username {
                        state.service.get_router().get_user_addrs(&username)
                    } else {
                        if let Some(addr) = query.addr {
                            vec![addr]
                        } else {
                            return StatusCode::NOT_FOUND.into_response();
                        }
                    };

                    let mut res = Vec::with_capacity(addrs.len());
                    for addr in addrs {
                        if let Some(node) = state.service.get_router().get_socket(&Arc::new(addr)) {
                            res.push(json!({
                                "address": addr,
                                "username": node.username,
                                "password": node.password,
                                "allocated_channels": node.channels,
                                "allocated_port": node.port,
                                "expiration": node.expiration,
                                "lifetime": node.lifetime.elapsed().as_secs(),
                            }));
                        }
                    }

                    Json(Value::Array(res)).into_response()
                },
            ),
        )
        .route(
            "/session/statistics",
            get(
                |Query(query): Query<QueryFilter>, State(state): State<Arc<AppState>>| async move {
                    if let Some(addr) = query.addr {
                        if let Some(counts) = state.statistics.get(&addr) {
                            return Json(json!({
                                "received_bytes": counts.received_bytes,
                                "send_bytes": counts.send_bytes,
                                "received_pkts": counts.received_pkts,
                                "send_pkts": counts.send_pkts,
                                "error_pkts": counts.error_pkts,
                            }))
                            .into_response();
                        }
                    }

                    StatusCode::NOT_FOUND.into_response()
                },
            ),
        )
        .route(
            "/session",
            delete(
                |Query(query): Query<QueryFilter>, State(state): State<Arc<AppState>>| async move {
                    let addrs = if let Some(username) = query.username {
                        state.service.get_router().get_user_addrs(&username)
                    } else {
                        if let Some(addr) = query.addr {
                            vec![addr]
                        } else {
                            return StatusCode::NOT_FOUND;
                        }
                    };

                    for addr in addrs {
                        if state.service.get_router().remove(&Arc::new(addr)).is_none() {
                            return StatusCode::EXPECTATION_FAILED;
                        }
                    }

                    StatusCode::OK
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
                headers.insert("Rid", HeaderValue::from_str(&RID).unwrap());
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
        headers.insert("Rid", HeaderValue::from_str(&RID)?);

        let client = Arc::new(
            ClientBuilder::new()
                .default_headers(headers)
                .timeout(Duration::from_secs(5))
                .build()?,
        );

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

    pub async fn get_password(&self, addr: &SocketAddr, name: &str) -> Option<String> {
        if let Some(pwd) = self.config.auth.static_credentials.get(name) {
            return Some(pwd.clone());
        }

        if let Some(secret) = &self.config.auth.static_auth_secret {
            return encode_password(secret, name);
        }

        if let Some(server) = &self.config.api.hooks {
            if let Ok(res) = self
                .client
                .get(format!("{}/password?addr={}&name={}", server, addr, name))
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

    pub fn send_event(&self, event: Value) {
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
