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
    balance_request::Payload, balance_response::Reply, BalanceRequest, BalanceResponse,
    GetCandidateReply, Host,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Arc::new(Config::load()?);
    let cluster = Cluster::new(cfg.clone());
    let socket = Arc::new(UdpSocket::bind(cfg.net.bind).await?);

    if let Some(superiors) = cfg.cluster.superiors {
        let socket = socket.clone();
        tokio::spawn(async move {
            let mut ping_buf = BytesMut::new();
            let _ = BalanceRequest {
                id: 0,
                payload: Some(Payload::Ping(())),
            }
            .encode(&mut ping_buf);

            loop {
                sleep(Duration::from_secs(10)).await;
                if let Err(e) = socket.send_to(&ping_buf, superiors).await {
                    if e.kind() != ConnectionReset {
                        break;
                    }
                }
            }
        });
    }

    let mut buf = [0u8; 40960];
    let mut send_buf = BytesMut::new();

    while let Ok((size, addr)) = socket.recv_from(&mut buf).await {
        if let Ok(req) = BalanceRequest::decode(&buf[..size]) {
            if let Some(payload) = req.payload {
                match payload {
                    Payload::Candidates(_) => {
                        send_buf.clear();
                        BalanceResponse {
                            id: req.id,
                            reply: Some(Reply::Candidate(GetCandidateReply {
                                hosts: cluster
                                    .get_onlines()
                                    .iter()
                                    .map(|v| Host {
                                        ip: v.ip().to_string(),
                                        port: v.port() as u32,
                                    })
                                    .collect(),
                            })),
                        }
                        .encode(&mut send_buf)?;
                        if let Err(e) = socket.send_to(&send_buf, addr).await {
                            if e.kind() != ConnectionReset {
                                break;
                            }
                        }
                    }
                    Payload::Ping(_) => {
                        cluster.update(&addr);
                    }
                    Payload::Probe(_) => {
                        send_buf.clear();
                        BalanceResponse {
                            id: req.id,
                            reply: Some(Reply::Probe(ProbeReply {
                                hosts: cfg
                                    .cluster
                                    .nodes
                                    .iter()
                                    .map(|v| Host {
                                        ip: v.ip().to_string(),
                                        port: v.port() as u32,
                                    })
                                    .collect(),
                            })),
                        }
                        .encode(&mut send_buf)?;
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

    Ok(())
}
