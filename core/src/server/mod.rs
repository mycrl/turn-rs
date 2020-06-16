use tokio::net::TcpListener;
use std::net::SocketAddr;
use std::io::Error;

pub async fn run(addr: SocketAddr) -> Result<(), Error> {
    let mut listener = TcpListener::bind(addr).await?;
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            
        });
    }

    Ok(())
}
