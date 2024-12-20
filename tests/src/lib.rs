use base64::prelude::*;
use bytes::BytesMut;
use once_cell::sync::Lazy;
use stun::{
    attribute::{
        ChannelNumber, Data, ErrKind, ErrorCode, Lifetime, MappedAddress, Nonce, Realm, ReqeestedTransport, ResponseOrigin, Transport, UserName, XorMappedAddress, XorPeerAddress, XorRelayedAddress
    },
    Decoder, Kind, MessageReader, MessageWriter, Method, Payload,
};

use rand::seq::SliceRandom;
use tokio::{net::UdpSocket, runtime::Runtime};
use turn_server::{
    config::{self, *},
    startup,
};

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    thread::sleep,
    time::Duration,
};

static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());

static BIND_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 31, 62));
static USERNAME: &str = "user1";
static PASSWORD: &str = "test";
static REALM: &str = "localhost";

#[derive(Debug, Clone)]
pub enum AuthMethod {
    Static,
    Secret(String),
    Hooks(String),
}

pub fn create_turn_server(auth_method: &AuthMethod, bind: SocketAddr) {
    let mut static_credentials = HashMap::new();
    let mut static_auth_secret = None;
    let mut api = Api::default();

    match auth_method {
        AuthMethod::Static => {
            static_credentials.insert(USERNAME.to_string(), PASSWORD.to_string());
        }
        AuthMethod::Secret(secret) => {
            static_auth_secret = Some(secret.clone());
        }
        AuthMethod::Hooks(uri) => {
            api.hooks = Some(uri.clone());
        }
    };

    RUNTIME.spawn(async move {
        startup(Arc::new(Config {
            auth: Auth {
                static_credentials,
                static_auth_secret,
            },
            api: Api::default(),
            log: Log::default(),
            turn: Turn {
                realm: REALM.to_string(),
                interfaces: vec![Interface {
                    transport: config::Transport::UDP,
                    external: bind,
                    bind,
                }],
            },
        }))
        .await
        .unwrap();
    });

    sleep(Duration::from_secs(2));
}

pub struct TurnClient {
    key_buf: [u8; 16],
    decoder: Decoder,
    client: UdpSocket,
    token_buf: [u8; 12],
    recv_buf: [u8; 1500],
    send_buf: BytesMut,
    bind_request_buf: BytesMut,
    base_allocate_request_buf: BytesMut,
    bind: SocketAddr,
    nonce: Option<String>,
}

impl TurnClient {
    pub fn new(auth_method: &AuthMethod, bind: SocketAddr, username: &str) -> Self {
        let client = RUNTIME
            .block_on(UdpSocket::bind("0.0.0.0:0"))
            .unwrap();

        RUNTIME.block_on(client.connect(bind)).unwrap();

        let key = match &auth_method {
            AuthMethod::Static => PASSWORD.to_string(),
            AuthMethod::Secret(secret) => Self::encode_password(secret, username).unwrap(),
            AuthMethod::Hooks(_) => PASSWORD.to_string(),
        };

        let key_buf = stun::util::long_key(username, &key, REALM);

        let token_buf = {
            let mut rng = rand::thread_rng();
            let mut token = [0u8; 12];
            token.shuffle(&mut rng);
            token
        };

        let bind_request_buf = {
            let mut buf = BytesMut::with_capacity(1500);
            let mut msg = MessageWriter::new(Method::Binding(Kind::Request), &token_buf, &mut buf);

            msg.flush(None).unwrap();
            buf
        };

        let base_allocate_request_buf = {
            let mut buf = BytesMut::with_capacity(1500);
            let mut msg = MessageWriter::new(Method::Allocate(Kind::Request), &token_buf, &mut buf);

            msg.append::<ReqeestedTransport>(Transport::UDP);
            msg.flush(None).unwrap();
            buf
        };

        Self {
            key_buf,
            bind_request_buf,
            base_allocate_request_buf,
            send_buf: BytesMut::with_capacity(2048),
            recv_buf: [0u8; 1500],
            decoder: Decoder::default(),
            token_buf,
            client,
            bind,
            nonce: None,
        }
    }

    pub fn binding_request(&mut self) {
        RUNTIME
            .block_on(self.client.send(&self.bind_request_buf))
            .unwrap();

        let size = RUNTIME
            .block_on(self.client.recv(&mut self.recv_buf))
            .unwrap();

        let ret = self.decoder.decode(&self.recv_buf[..size]).unwrap();
        let ret = Self::get_message_from_payload(ret);

        assert_eq!(ret.method, Method::Binding(Kind::Response));
        assert_eq!(ret.token, self.token_buf.as_slice());

        let value = ret.get::<XorMappedAddress>().unwrap();
        assert_eq!(value, self.client.local_addr().unwrap());

        let value = ret.get::<MappedAddress>().unwrap();
        assert_eq!(value, self.client.local_addr().unwrap());

        let value = ret.get::<ResponseOrigin>().unwrap();
        assert_eq!(value, self.bind);
    }

    pub fn base_allocate_request(&mut self) {
        RUNTIME
            .block_on(self.client.send(&self.base_allocate_request_buf))
            .unwrap();

        let size = RUNTIME
            .block_on(self.client.recv(&mut self.recv_buf))
            .unwrap();
        let ret = self.decoder.decode(&self.recv_buf[..size]).unwrap();
        let ret = Self::get_message_from_payload(ret);

        assert_eq!(ret.method, Method::Allocate(Kind::Error));
        assert_eq!(ret.token, self.token_buf.as_slice());

        let value = ret.get::<ErrorCode>().unwrap();
        assert_eq!(value.code, ErrKind::Unauthorized as u16);

        let value = ret.get::<Realm>().unwrap();
        assert_eq!(value, REALM);

        self.nonce = Some(ret.get::<Nonce>().unwrap().to_string());
    }

    pub fn allocate_request(&mut self) -> u16 {
        let mut buf = BytesMut::with_capacity(1500);
        let mut msg = MessageWriter::new(Method::Allocate(Kind::Request), &self.token_buf, &mut buf);

        msg.append::<ReqeestedTransport>(Transport::UDP);
        msg.append::<UserName>(USERNAME);
        msg.append::<Realm>(REALM);
        msg.append::<Nonce>(self.nonce.as_ref().unwrap());
        msg.flush(Some(&self.key_buf)).unwrap();

        RUNTIME
            .block_on(self.client.send(&buf))
            .unwrap();

        let size = RUNTIME
            .block_on(self.client.recv(&mut self.recv_buf))
            .unwrap();
        let ret = self.decoder.decode(&self.recv_buf[..size]).unwrap();
        let ret = Self::get_message_from_payload(ret);

        assert_eq!(ret.method, Method::Allocate(Kind::Response));
        assert_eq!(ret.token, self.token_buf.as_slice());
        ret.integrity(&self.key_buf).unwrap();

        let relay = ret.get::<XorRelayedAddress>().unwrap();
        assert_eq!(relay.ip(), BIND_IP);

        let value = ret.get::<XorMappedAddress>().unwrap();
        assert_eq!(value, self.client.local_addr().unwrap());

        let value = ret.get::<Lifetime>().unwrap();
        assert_eq!(value, 600);

        relay.port()
    }

    pub fn create_permission_request(&mut self, username: &str) {
        let mut msg = MessageWriter::new(
            Method::CreatePermission(Kind::Request),
            &self.token_buf,
            &mut self.send_buf,
        );

        msg.append::<XorPeerAddress>(SocketAddr::new(BIND_IP, 80));
        msg.append::<UserName>(username);
        msg.append::<Realm>(REALM);
        msg.append::<Nonce>(self.nonce.as_ref().unwrap());
        msg.flush(Some(&self.key_buf)).unwrap();
        RUNTIME.block_on(self.client.send(&self.send_buf)).unwrap();

        let size = RUNTIME
            .block_on(self.client.recv(&mut self.recv_buf))
            .unwrap();
        let ret = self.decoder.decode(&self.recv_buf[..size]).unwrap();
        let ret = Self::get_message_from_payload(ret);

        assert_eq!(ret.method, Method::CreatePermission(Kind::Response));
        assert_eq!(ret.token, self.token_buf.as_slice());
        ret.integrity(&self.key_buf).unwrap();
    }

    pub fn channel_bind_request(&mut self, port: u16, username: &str) {
        let mut msg = MessageWriter::new(
            Method::ChannelBind(Kind::Request),
            &self.token_buf,
            &mut self.send_buf,
        );

        msg.append::<ChannelNumber>(0x4000);
        msg.append::<XorPeerAddress>(SocketAddr::new(BIND_IP, port));
        msg.append::<UserName>(username);
        msg.append::<Realm>(REALM);
        msg.append::<Nonce>(self.nonce.as_ref().unwrap());
        msg.flush(Some(&self.key_buf)).unwrap();
        RUNTIME.block_on(self.client.send(&self.send_buf)).unwrap();

        let size = RUNTIME
            .block_on(self.client.recv(&mut self.recv_buf))
            .unwrap();
        let ret = self.decoder.decode(&self.recv_buf[..size]).unwrap();
        let ret = Self::get_message_from_payload(ret);

        assert_eq!(ret.method, Method::ChannelBind(Kind::Response));
        assert_eq!(ret.token, self.token_buf.as_slice());
        ret.integrity(&self.key_buf).unwrap();
    }

    pub fn refresh_request(&mut self, username: &str) {
        let mut msg = MessageWriter::new(
            Method::Refresh(Kind::Request),
            &self.token_buf,
            &mut self.send_buf,
        );

        msg.append::<Lifetime>(0);
        msg.append::<UserName>(username);
        msg.append::<Realm>(REALM);
        msg.append::<Nonce>(self.nonce.as_ref().unwrap());
        msg.flush(Some(&self.key_buf)).unwrap();
        RUNTIME.block_on(self.client.send(&self.send_buf)).unwrap();

        let size = RUNTIME
            .block_on(self.client.recv(&mut self.recv_buf))
            .unwrap();
        let ret = self.decoder.decode(&self.recv_buf[..size]).unwrap();
        let ret = Self::get_message_from_payload(ret);

        assert_eq!(ret.method, Method::Refresh(Kind::Response));
        assert_eq!(ret.token, self.token_buf.as_slice());
        ret.integrity(&self.key_buf).unwrap();

        let value = ret.get::<Lifetime>().unwrap();
        assert_eq!(value, 0);
    }

    pub fn indication(&mut self, peer: &Self, port: u16) {
        let mut msg =
            MessageWriter::new(Method::SendIndication, &self.token_buf, &mut self.send_buf);

        msg.append::<XorPeerAddress>(SocketAddr::new(BIND_IP, port));
        msg.append::<Data>(self.token_buf.as_slice());
        msg.flush(None).unwrap();
        RUNTIME.block_on(self.client.send(&self.send_buf)).unwrap();

        let size = RUNTIME
            .block_on(peer.client.recv(&mut self.recv_buf))
            .unwrap();
        let ret = self.decoder.decode(&self.recv_buf[..size]).unwrap();
        let ret = Self::get_message_from_payload(ret);

        assert_eq!(ret.method, Method::DataIndication);
        assert_eq!(ret.token, self.token_buf.as_slice());

        let value = ret.get::<Data>().unwrap();
        assert_eq!(value, self.token_buf.as_slice());
    }

    fn get_message_from_payload<'a>(payload: Payload<'a>) -> MessageReader<'a> {
        if let Payload::Message(m) = payload {
            m
        } else {
            panic!("get message from payload failed!")
        }
    }

    fn encode_password(key: &str, username: &str) -> Option<String> {
        Some(
            BASE64_STANDARD.encode(
                stun::util::hmac_sha1(key.as_bytes(), &[username.as_bytes()])
                    .ok()?
                    .into_bytes()
                    .as_slice(),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::SocketAddr,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{create_turn_server, AuthMethod, TurnClient, BIND_IP, PASSWORD, USERNAME};

    fn get_current_timestamp_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    fn get_current_timestamp_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    // #[test]
    // fn static_auth_testing() {
    //     let bind = SocketAddr::new(BIND_IP, 4478);
    //     let auth = AuthMethod::Static;

    //     create_turn_server(&auth, bind);

    //     let mut local = TurnClient::new(&auth, bind, USERNAME);
    //     local.binding_request();
    //     local.base_allocate_request();

    //     let port = local.allocate_request();
    //     local.create_permission_request(USERNAME);
    //     local.channel_bind_request(port, USERNAME);
    //     local.refresh_request(USERNAME);
    // }

    #[test]
    fn turn_rest_testing() {
        let bind = SocketAddr::new(BIND_IP, 3478);
        let auth = AuthMethod::Hooks(PASSWORD.to_string());

        // create_turn_server(&auth, bind);

        let mut local = TurnClient::new(&auth, bind, &USERNAME);
        local.binding_request();
        local.base_allocate_request();

        let port = local.allocate_request();
        let port = local.allocate_request();
        local.create_permission_request(&USERNAME);
        local.channel_bind_request(port, &USERNAME);
        local.refresh_request(&USERNAME);
    }
}
