mod cluster;
mod config;

use std::{io::ErrorKind::ConnectionReset, sync::Arc, time::Duration};

use cluster::Cluster;
use config::Config;
use prost::{bytes::BytesMut, Message};
use proto::ProbeReply;
use tokio::{net::UdpSocket, time::sleep};

mod proto {
    include!(concat!(env!("OUT_DIR"), "/balance.rs"));
}

use self::proto::{
    balance_request::Payload, balance_response::Reply, BalanceRequest, BalanceResponse, Host,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Arc::new(Config::load()?);
    simple_logger::init_with_level(cfg.log.level.as_level())?;

    let cluster = Cluster::new(cfg.clone());
    let socket = Arc::new(UdpSocket::bind(cfg.net.bind).await?);

    log::info!("balance server listening: addr={}", cfg.net.bind);

    for _ in 0..num_cpus::get() {
        let cluster = cluster.clone();
        let socket = socket.clone();
        let cfg = cfg.clone();

        tokio::spawn(async move {
            let mut buf = [0u8; 40960];
            let mut send_buf = BytesMut::new();

            loop {
                // Note: An error will also be reported when the remote host is shut down, which
                // is not processed yet, but a warning will be issued.
                let (size, addr) = match socket.recv_from(&mut buf).await {
                    Err(e) if e.kind() != ConnectionReset => break,
                    Ok(s) => s,
                    _ => continue,
                };

                if let Ok(req) = BalanceRequest::decode(&buf[..size]) {
                    if let Some(payload) = req.payload {
                        match payload {
                            // If it's a ping message, then just refresh the timer.
                            Payload::Ping(_) => {
                                cluster.update(&addr);
                            }
                            Payload::Probe(_) => {
                                // Clean up the last message encoded in the send buffer first.
                                send_buf.clear();

                                // Only subordinate nodes that are currently online are reported.
                                let onlines = cluster.get_onlines();
                                BalanceResponse {
                                    id: req.id,
                                    reply: Some(Reply::Probe(ProbeReply {
                                        // If the subordinate node is empty, it means that this
                                        // balance server is
                                        // the last level, and it is enough to report the
                                        // listening address of the turn server directly.
                                        turn: if onlines.is_empty() {
                                            cfg.turn.bind.map(|v| Host {
                                                ip: v.ip().to_string(),
                                                port: v.port() as u32,
                                            })
                                        } else {
                                            None
                                        },
                                        hosts: onlines
                                            .iter()
                                            .map(|v| Host {
                                                ip: v.ip().to_string(),
                                                port: v.port() as u32,
                                            })
                                            .collect(),
                                    })),
                                }
                                .encode(&mut send_buf)?;

                                // Note: An error will also be reported when the remote host is shut
                                // down, which is not processed yet,
                                // but a warning will be issued.
                                if let Err(e) = socket.send_to(&send_buf, addr).await {
                                    if e.kind() != ConnectionReset {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Ok::<(), anyhow::Error>(())
        });
    }

    // Enable timed pings if a superior balance server exists, for the purpose of
    // letting the superior know that I'm still alive.
    if let Some(superiors) = cfg.cluster.superiors {
        let mut ping_buf = BytesMut::new();
        let _ = BalanceRequest {
            id: 0,
            payload: Some(Payload::Ping(())),
        }
        .encode(&mut ping_buf);

        loop {
            if let Err(e) = socket.send_to(&ping_buf, superiors).await {
                if e.kind() != ConnectionReset {
                    break;
                }
            }
            
            // Sent every 10 seconds, too many packets can cause unnecessary overhead by the
            // parent.
            sleep(Duration::from_secs(10)).await;
        }
    } else {
        std::future::pending::<()>().await;
    }

    Ok(())
}
