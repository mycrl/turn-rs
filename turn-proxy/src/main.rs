use bytes::BytesMut;
use turn_proxy::rpc::transport::Protocol;
use tokio::{
    net::TcpListener,
    io::AsyncReadExt,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    tokio::spawn(async move {});

    let addr = "127.0.0.1:8081";
    let listener = TcpListener::bind(addr).await?;
    log::info!("tcp server listening: addr={}", addr);

    while let Ok((mut socket, remote_addr)) = listener.accept().await {
        log::info!("new node connected: addr={}", remote_addr);

        tokio::spawn(async move {
            let (mut reader, mut _writer) = socket.split();
            let mut bytes = BytesMut::new();

            while let Ok(_) = reader.read_buf(&mut bytes).await {
                match Protocol::decode(&bytes[..]) {
                    Ok(ret) => {
                        if let Some(ret) = ret {
                            // handle recv result.
                            let _ = bytes.split_to(ret.size);
                        }
                    },
                    Err(_) => {
                        break;
                    },
                }
            }

            log::info!("node disconnect: addr={}", remote_addr);
        });
    }

    Ok(())
}
