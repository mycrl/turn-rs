use bytes::BytesMut;
use faster_stun::attribute::{
    ChannelNumber, Data, ErrKind, ErrorCode, Lifetime, MappedAddress, Realm, ReqeestedTransport,
    ResponseOrigin, Transport, UserName, XorMappedAddress, XorPeerAddress, XorRelayedAddress,
};

use faster_stun::{Decoder, Kind, MessageReader, MessageWriter, Method, Payload};
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use tokio::net::UdpSocket;
use turn_server::{
    config::{self, *},
    server_main,
};

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

/// global static var

pub const BIND_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
pub const BIND_ADDR: SocketAddr = SocketAddr::new(BIND_IP, 3478);
pub const USERNAME: &'static str = "user1";
pub const PASSWORD: &'static str = "test";
pub const REALM: &'static str = "localhost";

static mut RECV_BUF: [u8; 1500] = [0u8; 1500];
static mut SEND_BUF: Lazy<BytesMut> = Lazy::new(|| BytesMut::with_capacity(2048));
static TOKEN_BUF: Lazy<[u8; 12]> = Lazy::new(|| {
    let mut rng = rand::thread_rng();
    let mut token = [0u8; 12];
    token.shuffle(&mut rng);
    token
});

static KEY_BUF: Lazy<[u8; 16]> =
    Lazy::new(|| faster_stun::util::long_key(USERNAME, PASSWORD, REALM));
static mut DECODER: Lazy<Decoder> = Lazy::new(|| Decoder::new());

/// global static var end

fn get_message_from_payload<'a, 'b>(payload: Payload<'a, 'b>) -> MessageReader<'a, 'b> {
    if let Payload::Message(m) = payload {
        m
    } else {
        panic!("get message from payload failed!")
    }
}

pub async fn create_turn() {
    let mut auth = HashMap::new();
    auth.insert(USERNAME.to_string(), PASSWORD.to_string());

    // Because it is testing, it is not reasonable to start a separate process
    // to start turn-server, and the configuration file is not convenient to
    // pass, so turn-server is used as a library here, and the server is
    // started with a custom configuration.
    tokio::spawn(async move {
        server_main(Arc::new(Config {
            auth,
            controller: Controller::default(),
            hooks: Hooks::default(),
            log: Log::default(),
            turn: Turn {
                realm: REALM.to_string(),
                interfaces: vec![Interface {
                    transport: config::Transport::UDP,
                    bind: BIND_ADDR,
                    external: BIND_ADDR,
                }],
            },
        }))
        .await
        .unwrap();
    });
}

// Create a udp connection and connect to the turn-server, and then start
// the corresponding session process checks in sequence. It should be noted
// that the order of request responses is relatively strict, and should not
// be changed under normal circumstances.
pub async fn create_client() -> UdpSocket {
    let socket = UdpSocket::bind(SocketAddr::new(BIND_IP, 0)).await.unwrap();
    socket.connect(BIND_ADDR).await.unwrap();
    socket
}

static BIND_REQUEST_BUF: Lazy<BytesMut> = Lazy::new(|| {
    let mut buf = BytesMut::with_capacity(1500);
    let mut msg = MessageWriter::new(Method::Binding(Kind::Request), &TOKEN_BUF, &mut buf);

    msg.flush(None).unwrap();
    buf
});

pub async fn binding_request(socket: &UdpSocket) {
    socket.send_to(&BIND_REQUEST_BUF, BIND_ADDR).await.unwrap();
    let size = socket.recv(unsafe { &mut RECV_BUF }).await.unwrap();

    let decoder = unsafe { &mut DECODER };
    let ret = decoder.decode(unsafe { &RECV_BUF[..size] }).unwrap();
    let ret = get_message_from_payload(ret);

    assert_eq!(ret.method, Method::Binding(Kind::Response));
    assert_eq!(ret.token, TOKEN_BUF.as_slice());

    let value = ret.get::<XorMappedAddress>().unwrap();
    assert_eq!(value, socket.local_addr().unwrap());

    let value = ret.get::<MappedAddress>().unwrap();
    assert_eq!(value, socket.local_addr().unwrap());

    let value = ret.get::<ResponseOrigin>().unwrap();
    assert_eq!(value, BIND_ADDR);
}

static BASE_ALLOCATE_REQUEST_BUF: Lazy<BytesMut> = Lazy::new(|| {
    let mut buf = BytesMut::with_capacity(1500);
    let mut msg = MessageWriter::new(Method::Allocate(Kind::Request), &TOKEN_BUF, &mut buf);

    msg.append::<ReqeestedTransport>(Transport::UDP);
    msg.flush(None).unwrap();
    buf
});

pub async fn base_allocate_request(socket: &UdpSocket) {
    socket
        .send_to(&BASE_ALLOCATE_REQUEST_BUF, BIND_ADDR)
        .await
        .unwrap();

    let decoder = unsafe { &mut DECODER };
    let size = socket.recv(unsafe { &mut RECV_BUF }).await.unwrap();
    let ret = decoder.decode(unsafe { &RECV_BUF[..size] }).unwrap();
    let ret = get_message_from_payload(ret);

    assert_eq!(ret.method, Method::Allocate(Kind::Error));
    assert_eq!(ret.token, TOKEN_BUF.as_slice());

    let value = ret.get::<ErrorCode>().unwrap();
    assert_eq!(value.code, ErrKind::Unauthorized as u16);

    let value = ret.get::<Realm>().unwrap();
    assert_eq!(value, REALM);
}

static ALLOCATE_REQUEST_BUF: Lazy<BytesMut> = Lazy::new(|| {
    let mut buf = BytesMut::with_capacity(1500);
    let mut msg = MessageWriter::new(Method::Allocate(Kind::Request), &TOKEN_BUF, &mut buf);

    msg.append::<ReqeestedTransport>(Transport::UDP);
    msg.append::<UserName>(USERNAME);
    msg.append::<Realm>(REALM);
    msg.flush(Some(&KEY_BUF)).unwrap();
    buf
});

pub async fn allocate_request(socket: &UdpSocket) -> u16 {
    socket
        .send_to(&ALLOCATE_REQUEST_BUF, BIND_ADDR)
        .await
        .unwrap();

    let decoder = unsafe { &mut DECODER };
    let size = socket.recv(unsafe { &mut RECV_BUF }).await.unwrap();
    let ret = decoder.decode(unsafe { &RECV_BUF[..size] }).unwrap();
    let ret = get_message_from_payload(ret);

    assert_eq!(ret.method, Method::Allocate(Kind::Response));
    assert_eq!(ret.token, TOKEN_BUF.as_slice());
    ret.integrity(&KEY_BUF).unwrap();

    let relay = ret.get::<XorRelayedAddress>().unwrap();
    assert_eq!(relay.ip(), BIND_IP);

    let value = ret.get::<XorMappedAddress>().unwrap();
    assert_eq!(value, socket.local_addr().unwrap());

    let value = ret.get::<Lifetime>().unwrap();
    assert_eq!(value, 600);

    relay.port()
}

pub async fn create_permission_request(socket: &UdpSocket, port: u16) {
    let mut msg = MessageWriter::new(
        Method::CreatePermission(Kind::Request),
        &TOKEN_BUF,
        unsafe { &mut SEND_BUF },
    );

    msg.append::<XorPeerAddress>(SocketAddr::new(BIND_IP, port));
    msg.append::<UserName>(USERNAME);
    msg.append::<Realm>(REALM);
    msg.flush(Some(&KEY_BUF)).unwrap();
    socket
        .send_to(unsafe { &SEND_BUF }, BIND_ADDR)
        .await
        .unwrap();

    let decoder = unsafe { &mut DECODER };
    let size = socket.recv(unsafe { &mut RECV_BUF }).await.unwrap();
    let ret = decoder.decode(unsafe { &RECV_BUF[..size] }).unwrap();
    let ret = get_message_from_payload(ret);

    assert_eq!(ret.method, Method::CreatePermission(Kind::Response));
    assert_eq!(ret.token, TOKEN_BUF.as_slice());
    ret.integrity(&KEY_BUF).unwrap();
}

pub async fn channel_bind_request(socket: &UdpSocket, port: u16) {
    let mut msg = MessageWriter::new(Method::ChannelBind(Kind::Request), &TOKEN_BUF, unsafe {
        &mut SEND_BUF
    });

    msg.append::<ChannelNumber>(0x4000);
    msg.append::<XorPeerAddress>(SocketAddr::new(BIND_IP, port));
    msg.append::<UserName>(USERNAME);
    msg.append::<Realm>(REALM);
    msg.flush(Some(&KEY_BUF)).unwrap();
    socket
        .send_to(unsafe { &SEND_BUF }, BIND_ADDR)
        .await
        .unwrap();

    let decoder = unsafe { &mut DECODER };
    let size = socket.recv(unsafe { &mut RECV_BUF }).await.unwrap();
    let ret = decoder.decode(unsafe { &RECV_BUF[..size] }).unwrap();
    let ret = get_message_from_payload(ret);

    assert_eq!(ret.method, Method::ChannelBind(Kind::Response));
    assert_eq!(ret.token, TOKEN_BUF.as_slice());
    ret.integrity(&KEY_BUF).unwrap();
}

pub async fn refresh_request(socket: &UdpSocket) {
    let mut msg = MessageWriter::new(Method::Refresh(Kind::Request), &TOKEN_BUF, unsafe {
        &mut SEND_BUF
    });

    msg.append::<Lifetime>(0);
    msg.append::<UserName>(USERNAME);
    msg.append::<Realm>(REALM);
    msg.flush(Some(&KEY_BUF)).unwrap();
    socket
        .send_to(unsafe { &SEND_BUF }, BIND_ADDR)
        .await
        .unwrap();

    let decoder = unsafe { &mut DECODER };
    let size = socket.recv(unsafe { &mut RECV_BUF }).await.unwrap();
    let ret = decoder.decode(unsafe { &RECV_BUF[..size] }).unwrap();
    let ret = get_message_from_payload(ret);

    assert_eq!(ret.method, Method::Refresh(Kind::Response));
    assert_eq!(ret.token, TOKEN_BUF.as_slice());
    ret.integrity(&KEY_BUF).unwrap();

    let value = ret.get::<Lifetime>().unwrap();
    assert_eq!(value, 0);
}

pub async fn indication(local: &UdpSocket, peer: &UdpSocket, port: u16) {
    let mut msg = MessageWriter::new(Method::SendIndication, &TOKEN_BUF, unsafe { &mut SEND_BUF });

    msg.append::<XorPeerAddress>(SocketAddr::new(BIND_IP, port));
    msg.append::<Data>(TOKEN_BUF.as_slice());
    msg.flush(None).unwrap();
    local
        .send_to(unsafe { &SEND_BUF }, BIND_ADDR)
        .await
        .unwrap();

    let decoder = unsafe { &mut DECODER };
    let size = peer.recv(unsafe { &mut RECV_BUF }).await.unwrap();
    let ret = decoder.decode(unsafe { &RECV_BUF[..size] }).unwrap();
    let ret = get_message_from_payload(ret);

    assert_eq!(ret.method, Method::DataIndication);
    assert_eq!(ret.token, TOKEN_BUF.as_slice());

    let value = ret.get::<Data>().unwrap();
    assert_eq!(value, TOKEN_BUF.as_slice());
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn integration_testing() {
        crate::create_turn().await;
        let socket = crate::create_client().await;
        crate::binding_request(&socket).await;
        crate::base_allocate_request(&socket).await;
        let port = crate::allocate_request(&socket).await;
        crate::create_permission_request(&socket, port).await;
        crate::channel_bind_request(&socket, port).await;
        crate::refresh_request(&socket).await;
    }
}
