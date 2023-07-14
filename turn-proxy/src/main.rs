mod config;

use std::sync::Arc;

use bytes::BytesMut;
use turn_proxy::rpc::{
    transport::Protocol,
    ProxyStateNotifyNode,
};

use tokio::{time::{sleep, Duration}, net::TcpStream, sync::Mutex};

struct ProxyNode {
    node: Mutex<ProxyStateNotifyNode>,
    socket: Mutex<Option<TcpStream>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Arc::new(config::Config::load()?);
    simple_logger::init_with_level(config.log.level.as_level())?;

    let mut nodes = Vec::with_capacity(config.nodes.len());
    for (index, node) in config.nodes.iter().enumerate() {
        nodes.push(ProxyNode {
            socket: Mutex::new(None),
            node: Mutex::new(ProxyStateNotifyNode {
                external: node.external,
                index: index as u8,
                addr: node.bind,
                online: false,
            }),
        })
    }

    let nodes = Arc::new(nodes);
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(config.net.recon_delay)).await;
            for node in nodes.iter() {
                let mut socket = node.socket.lock().await;
                if socket.is_none() {
                    let mut node = node.node.lock().await;
                    if let Ok(ret) = TcpStream::connect(node.addr).await {
                        let _ = socket.insert(ret);
                        node.online = true;
                    }
                }
            }
        }
    });

    Ok(())
}
