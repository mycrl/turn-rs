use tokio::net::UdpSocket;
use std::io::Error;
use stun_codec::MessageEncoder;
use stun_codec::MessageDecoder;
use stun_codec::rfc5389::Attribute;
use bytecodec::{DecodeExt, EncodeExt};
use stun_codec::MessageClass;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut socket = UdpSocket::bind("0.0.0.0:4378").await?;
    let mut decoder = MessageDecoder::<Attribute>::new();
    let mut encoder = MessageEncoder::<Attribute>::new();
    loop {
        let mut buffer = [0u8; 2048];
        let (size, addr) = socket.recv_from(&mut buffer).await?;
        let decoded = decoder.decode_from_bytes(&buffer[0..size]).unwrap().unwrap();
        println!("{:?}", addr);
        if decoded.attributes().count() == 0 {
            continue;
        }
    }
}
