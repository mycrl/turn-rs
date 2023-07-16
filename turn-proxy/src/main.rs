mod config;

use std::{
    sync::Arc,
    io::IoSlice,
};
use std::io::ErrorKind::ConnectionReset;
use bytes::{
    BytesMut,
    Bytes,
};
use turn_proxy::rpc::{
    transport::Protocol,
    ProxyStateNotifyNode,
    Payload,
};

use tokio::{
    time::{
        sleep,
        Duration,
    },
    net::{
        TcpStream,
        UdpSocket,
    },
    sync::{
        RwLock,
        mpsc::{
            UnboundedSender,
            UnboundedReceiver,
            unbounded_channel,
        },
        Mutex,
    },
    io::*,
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
                sender,
                receiver: Mutex::new(receiver),
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
                let state = nodes_[ret.to as usize].state.read().await;
                if let Err(e) = socket.send_to(ret.data, state.addr).await {
                    if e.kind() != ConnectionReset {
                        break;
                    }
                }
            }
        }

        log::error!("udp socket is closed!");
        std::process::exit(-1);
    });

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

        sleep(Duration::from_millis(config.net.recon_delay)).await;
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
                println!("====================");
                tokio::select! {
                    Ok(size) = socket.read_buf(&mut buf) => {
                        if size == 0 {
                            break;
                        }

                        if let Ok(ret) = Protocol::decode_head(&buf[..size]) {
                            if let Some((size, to)) = ret {
                                let data = buf.split_to(size);
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

                println!("==================== 1111");
            }

            println!("==================== 222");
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

    let payload: Vec<u8> = Payload::ProxyStateNotify(ret).into();
    println!("{}, {}", payload.len(), index);
    let head = Protocol::encode_header(&payload, index as u8);
    let vect = [IoSlice::new(&head), IoSlice::new(&payload)];
    socket.write_vectored(&vect).await?;

    Ok(())
}
