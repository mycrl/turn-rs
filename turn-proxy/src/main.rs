mod config;

use std::sync::Arc;
use std::io::{
    ErrorKind::ConnectionReset,
    IoSlice,
};

use bytes::{
    BytesMut,
    Bytes,
};

use turn_proxy::rpc::{
    transport::Protocol,
    ProxyStateNotifyNode,
    Request,
};

use tokio::io::*;
use tokio::time::{
    Duration,
    sleep,
};

use tokio::net::{
    TcpStream,
    UdpSocket,
};

use tokio::sync::{
    Mutex,
    RwLock,
};

use tokio::sync::mpsc::{
    UnboundedSender,
    UnboundedReceiver,
    unbounded_channel,
};

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
                external: node.external,
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
        let mut buf = [0u8; 4096];

        loop {
            let size = match socket.recv_from(&mut buf).await {
                Err(e) if e.kind() != ConnectionReset => break,
                Ok((s, a)) if cfg_nodes.iter().any(|node| node.bind == a) => s,
                _ => continue,
            };

            if let Ok(Some(ret)) = Protocol::decode(&buf[..size]) {
                if let Some(node) = nodes_.get(ret.to as usize) {
                    if let Err(e) = socket
                        .send_to(ret.data, node.state.read().await.addr)
                        .await
                    {
                        if e.kind() != ConnectionReset {
                            break;
                        }
                    }
                } else {
                    log::warn!(
                        "received a legtimate but insecure packet!: size={}, \
                         to={}",
                        ret.size,
                        ret.to
                    );
                }
            }
        }

        log::error!("udp socket is closed!");
        std::process::exit(-1);
    });

    let delay = Duration::from_millis(config.net.recon_delay);
    loop {
        for (i, node) in nodes.iter().enumerate() {
            let mut state = node.state.write().await;
            if !state.online {
                if let Ok(socket) = TcpStream::connect(state.addr).await {
                    log::info!("connected to proxy node: addr={}", state.addr);
                    on_tcp_socket(i, nodes.clone(), socket);
                    state.online = true;
                }
            }
        }

        sleep(delay).await;
    }
}

fn on_tcp_socket(
    index: usize,
    nodes: Arc<Vec<ProxyNode>>,
    mut socket: TcpStream,
) {
    tokio::spawn(async move {
        let remote_addr = socket
            .peer_addr()
            .expect("get socket remote socket is failed!");

        if send_state(index, &nodes, &mut socket).await.is_ok() {
            log::info!("send state to proxy node: addr={}", remote_addr);

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
        }

        nodes[index].state.write().await.online = false;
        log::info!("proxy node disconnect: addr={}", remote_addr);
    });
}

async fn send_state(
    index: usize,
    nodes: &Vec<ProxyNode>,
    socket: &mut TcpStream,
) -> Result<()> {
    let mut ret = Vec::with_capacity(nodes.len());
    for node in nodes {
        ret.push(node.state.read().await.clone());
    }

    let req: Vec<u8> = Request::ProxyStateNotify(ret).into();
    let head = Protocol::encode_header(&req, index as u8);
    let vect = [IoSlice::new(&head), IoSlice::new(&req)];
    socket.write_vectored(&vect).await?;
    Ok(())
}
