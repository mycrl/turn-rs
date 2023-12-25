use prost::Message;
use proto::BalanceRequestType;
use tokio::net::UdpSocket;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/balance.rs"));
}

use self::proto::BalanceRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:4000").await?;
    let mut buf = [0u8; 4096];

    while let Ok((size, addr)) = socket.recv_from(&mut buf).await {
        if let Ok(req) = BalanceRequest::decode(&buf[..size]) {
            if let Ok(r#type) = BalanceRequestType::try_from(req.r#type) {
                match r#type {
                    BalanceRequestType::GetCandidates => {
    
                    }
                    BalanceRequestType::Ping => {
                        
                    }
                }
            }
        }
    }

    Ok(())
}
