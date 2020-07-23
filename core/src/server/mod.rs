mod socket;

use std::io::Error;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt};
use bytes::BytesMut;

#[allow(warnings)]
pub async fn run(addr: SocketAddr) -> Result<(), Error> {
    let mut listener = TcpListener::bind(addr).await?;
    while let Ok((mut stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            loop {
                let mut buffer = [0u8; 2048];
                let size = stream.read(&mut buffer).await?;
                let chunk = BytesMut::from(&buffer[0..size]);
            }

            Ok::<(), Error>(())
        });
    }

    Ok(())
}
