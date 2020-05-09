mod codec;
mod server;

use std::error::Error;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     let mut server = Server::new(ServerAddress {
//         tcp: "0.0.0.0:1935".parse().unwrap(),
//         udp: "127.0.0.1:1936".parse().unwrap()
//     }).await?;

//     loop {
//         server.next().await;
//     }
// }

type Tx = tokio::sync::mpsc::UnboundedSender<bytes::Bytes>;
type Rx = tokio::sync::mpsc::UnboundedReceiver<bytes::Bytes>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let tcp_addr = "0.0.0.0:1935".parse::<std::net::SocketAddr>().unwrap();
    let udp_addr = "0.0.0.0:0".parse::<std::net::SocketAddr>().unwrap();
    let udp_to_addr = "127.0.0.1:1936".parse::<std::net::SocketAddr>().unwrap();
    let mut listener = tokio::net::TcpListener::bind(&tcp_addr).await?;
    let mut dgram = tokio::net::UdpSocket::bind(&udp_addr).await?;
    let (sender, mut receiver): (Tx, Rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(async move {
        loop {
            if let Ok(data) = receiver.try_recv() {
                let mut offset: usize = 0;
                loop {
                    let end = if offset + 1000 > data.len() { data.len() } else { offset + 1000 };
                    match dgram.send_to(&data[offset..end], &udp_to_addr).await {
                        Ok(size) => {
                            offset += size;
                            if &offset >= &data.len() { break; }
                        }, 
                        Err(e) => {
                            println!("send udp data err {:?}", e);
                        },
                    }
                }
            }
        }
    });

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(server::socket::Socket::<codec::rtmp::Rtmp>::new(stream, sender.clone()));
    }
}
