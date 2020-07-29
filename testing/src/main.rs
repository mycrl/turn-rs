use anyhow::Result;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use bytes::BytesMut;
use stun::STUN;

#[tokio::main]
#[allow(warnings)]
async fn main() -> Result<()> {
    let stun = STUN::new("127.0.0.1:3478".parse::<SocketAddr>().unwrap(), "quasipaa.lbxpz.com".to_string());
    let mut socket = UdpSocket::bind("0.0.0.0:3478").await?;
    loop { 
        let mut receiver = [0u8; 2048];
        let (size, addr) = socket.recv_from(&mut receiver).await?;
        let chunk = BytesMut::from(&receiver[0..size]);
        if let Some(res) = stun.process(chunk, addr) {
            socket.send_to(&res, addr).await?;
        }
    }

    Ok(())
}
