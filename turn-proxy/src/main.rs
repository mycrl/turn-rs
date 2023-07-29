mod config;

use std::io::ErrorKind::ConnectionReset;
use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use tokio::io::*;
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration};
use turn_proxy::rpc::{transport::Protocol, ProxyStateNotifyNode, Request};

struct Channel {
    receiver: Mutex<UnboundedReceiver<Bytes>>,
    sender: UnboundedSender<Bytes>,
}

struct ProxyNode {
    tcp: Channel,
    state: RwLock<ProxyStateNotifyNode>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Arc::new(config::Config::load()?);
    simple_logger::init_with_level(config.log.level.as_level())?;

    let mut nodes = Vec::with_capacity(config.nodes.len());
    for (index, node) in config.nodes.iter().enumerate() {
        let (sender, receiver) = unbounded_channel();
        nodes.push(ProxyNode {
            tcp: Channel {
                receiver: Mutex::new(receiver),
                sender,
            },
            state: RwLock::new(ProxyStateNotifyNode {
                externals: node.externals.clone(),
                index: index as u8,
                addr: node.bind,
                online: false,
            }),
        })
    }

    let nodes = Arc::new(nodes);
    let nodes_ = nodes.clone();

    let cfg_nodes = config.nodes.clone();
    let socket = UdpSocket::bind(&config.net.bind).await?;
    log::info!("udp socket bind: addr={}", config.net.bind);

    tokio::spawn(async move {
        let mut buf = vec![0u8; 4096];

        loop {
            let size = match socket.recv_from(&mut buf).await {
                Err(e) if e.kind() != ConnectionReset => break,
                Ok((s, a)) if cfg_nodes.iter().any(|node| node.bind == a) => s,
                _ => continue,
            };

            if let Ok(Some((_, to))) = Protocol::decode_head(&buf[..size]) {
                if let Some(node) = nodes_.get(to as usize) {
                    let addr = node.state.read().await.addr;
                    if let Err(e) = socket.send_to(&buf[..size], addr).await {
                        if e.kind() != ConnectionReset {
                            break;
                        }
                    }
                }
            }
        }

        log::error!("udp socket is closed!");
        std::process::exit(-1);
    });

    let delay = Duration::from_millis(config.net.recon_delay);
    loop {
        let mut is_ok = false;

        for (i, node) in nodes.iter().enumerate() {
            let mut state = node.state.write().await;
            if !state.online {
                if let Ok(socket) = TcpStream::connect(state.addr).await {
                    log::info!("connected to proxy node: addr={}", state.addr);

                    on_tcp_socket(i, nodes.clone(), socket);
                    state.online = true;
                    is_ok = true;
                }
            }
        }

        if is_ok {
            send_state(&nodes).await;
        }

        sleep(delay).await;
    }
}

fn on_tcp_socket(index: usize, nodes: Arc<Vec<ProxyNode>>, mut socket: TcpStream) {
    tokio::spawn(async move {
        let remote_addr = socket
            .peer_addr()
            .expect("get socket remote socket is failed!");

        let mut receiver = nodes[index].tcp.receiver.lock().await;
        let mut buf = BytesMut::new();

        loop {
            tokio::select! {
                ret = socket.read_buf(&mut buf) => {
                    let size = if let Ok(size) = ret {
                        if size == 0 {
                            break;
                        }

                        size
                    } else {
                        break;
                    };

                    if let Ok(ret) = Protocol::decode_head(&buf[..size]) {
                        if let Some((size, to)) = ret {
                            let data = buf.split_to(size).split_off(4);
                            if let Some(node) = nodes.get(to as usize) {
                                if node.tcp.sender.send(data.freeze()).is_err() {
                                    break;
                                }
                            }
                        }
                    } else {
                        break;
                    }
                },
                Some(ret) = receiver.recv() => {
                    if socket.write_all(&ret).await.is_err() {
                        break;
                    }
                },
                else => {
                    break;
                }
            }
        }

        {
            nodes[index].state.write().await.online = false;
            log::info!("proxy node disconnect: addr={}", remote_addr);
        }

        send_state(&nodes).await;
    });
}

async fn send_state(nodes: &Vec<ProxyNode>) {
    let mut ret = Vec::with_capacity(nodes.len());
    for node in nodes {
        ret.push(node.state.read().await.clone());
    }

    let req: Vec<u8> = Request::ProxyStateNotify(ret).into();
    for node in nodes {
        let state = node.state.read().await;
        if !state.online {
            continue;
        }

        let mut buf = BytesMut::new();
        buf.extend_from_slice(&Protocol::encode_header(&req, state.index));
        buf.extend_from_slice(&req);

        if node.tcp.sender.send(buf.freeze()).is_err() {
            log::error!("send to tcp channel failed!");
        }
    }
}
