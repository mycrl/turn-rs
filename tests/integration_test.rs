use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, LazyLock},
    time::Duration,
};

use anyhow::{Ok, Result, ensure};
use base64::{Engine, prelude::BASE64_STANDARD};
use bytes::BytesMut;
use rand::seq::SliceRandom;
use tokio::{
    net::UdpSocket,
    time::{sleep, timeout},
};

use turn_server::{
    config::{Api, Auth, Config, Interface, Log, Transport as TurnTransport, Turn},
    startup,
    stun::{
        ChannelData, Decoder, MessageEncoder, MessageRef, Payload,
        attribute::{
            ChannelNumber, Data, ErrorCode, ErrorKind, Lifetime, MappedAddress, Nonce, Realm, ReqeestedTransport,
            ResponseOrigin, Transport, UserName, XorMappedAddress, XorPeerAddress, XorRelayedAddress,
        },
        method::{
            ALLOCATE_ERROR, ALLOCATE_REQUEST, ALLOCATE_RESPONSE, BINDING_REQUEST, BINDING_RESPONSE,
            CHANNEL_BIND_REQUEST, CHANNEL_BIND_RESPONSE, CREATE_PERMISSION_REQUEST, CREATE_PERMISSION_RESPONSE,
            DATA_INDICATION, REFRESH_REQUEST, REFRESH_RESPONSE, SEND_INDICATION, StunMethod,
        },
        util::{hmac_sha1, long_term_credential_digest},
    },
};

static TOKEN: LazyLock<[u8; 12]> = LazyLock::new(|| {
    let mut rng = rand::thread_rng();
    let mut token = [0u8; 12];
    token.shuffle(&mut rng);
    token
});

pub async fn create_turn_server(bind: SocketAddr, auth: Auth, api: Api) -> Result<()> {
    tokio::spawn(async move {
        startup(Arc::new(Config {
            log: Log::default(),
            turn: Turn {
                realm: "localhost".to_string(),
                interfaces: vec![Interface {
                    transport: TurnTransport::UDP,
                    external: bind,
                    bind,
                }],
            },
            auth,
            api,
        }))
        .await
        .unwrap();
    });

    sleep(Duration::from_secs(3)).await;
    Ok(())
}

struct Operationer {
    decoder: Decoder,
    socket: UdpSocket,
    recv_bytes: [u8; 1500],
    send_bytes: BytesMut,
}

impl Operationer {
    async fn new(server: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind("127.0.0.1:0").await?;
        socket.connect(server).await?;

        Ok(Self {
            send_bytes: BytesMut::with_capacity(1500),
            decoder: Decoder::default(),
            recv_bytes: [0u8; 1500],
            socket,
        })
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    fn create_message(&mut self, method: StunMethod) -> MessageEncoder {
        MessageEncoder::new(method, &TOKEN, &mut self.send_bytes)
    }

    fn create_channel_data(&mut self, number: u16, bytes: &[u8]) {
        ChannelData { number, bytes }.encode(&mut self.send_bytes);
    }

    async fn send(&self) -> Result<()> {
        self.socket.send(&self.send_bytes).await?;
        Ok(())
    }

    async fn read_message(&mut self) -> Result<MessageRef> {
        let size = timeout(Duration::from_secs(1), self.socket.recv(&mut self.recv_bytes)).await??;

        if let Payload::Message(message) = self.decoder.decode(&self.recv_bytes[..size])? {
            if message.token() != TOKEN.as_slice() {
                Err(anyhow::anyhow!("Message token does not match"))
            } else {
                Ok(message)
            }
        } else {
            Err(anyhow::anyhow!("payload not a message"))
        }
    }

    async fn read_channel_data(&mut self) -> Result<ChannelData> {
        let size = timeout(Duration::from_secs(1), self.socket.recv(&mut self.recv_bytes)).await??;

        if let Payload::ChannelData(channel_data) = self.decoder.decode(&self.recv_bytes[..size])? {
            Ok(channel_data)
        } else {
            Err(anyhow::anyhow!("payload not a channel data"))
        }
    }
}

pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Default)]
struct State {
    digest: [u8; 16],
    nonce: String,
    realm: String,
}

pub struct TurnClient {
    operationer: Operationer,
    credentials: Credentials,
    server: SocketAddr,
    state: State,
}

impl TurnClient {
    pub async fn new(server: SocketAddr, credentials: Credentials) -> Result<Self> {
        Ok(Self {
            operationer: Operationer::new(server).await?,
            state: State::default(),
            credentials,
            server,
        })
    }

    pub async fn binding(&mut self) -> Result<()> {
        {
            let mut message = self.operationer.create_message(BINDING_REQUEST);
            message.flush(None)?;

            self.operationer.send().await?;
        }

        let local_addr = self.operationer.local_addr()?;
        let message = self.operationer.read_message().await?;

        ensure!(message.method() == BINDING_RESPONSE);
        ensure!(message.get::<XorMappedAddress>() == Some(local_addr));
        ensure!(message.get::<MappedAddress>() == Some(local_addr));
        ensure!(message.get::<ResponseOrigin>() == Some(self.server));
        Ok(())
    }

    pub async fn allocate(&mut self) -> Result<u16> {
        {
            {
                let mut message = self.operationer.create_message(ALLOCATE_REQUEST);
                message.append::<ReqeestedTransport>(Transport::UDP);
                message.flush(None)?;

                self.operationer.send().await?;
            }

            let message = self.operationer.read_message().await?;

            ensure!(message.method() == ALLOCATE_ERROR);
            ensure!(message.get::<ErrorCode>().unwrap().code == ErrorKind::Unauthorized as u16);

            self.state.nonce = message.get::<Nonce>().unwrap().to_string();
            self.state.realm = message.get::<Realm>().unwrap().to_string();
            self.state.digest = long_term_credential_digest(
                &self.credentials.username,
                &self.credentials.password,
                &self.state.realm,
            );
        }

        {
            let mut message = self.operationer.create_message(ALLOCATE_REQUEST);
            message.append::<ReqeestedTransport>(Transport::UDP);
            message.append::<UserName>(&self.credentials.username);
            message.append::<Realm>(&self.state.realm);
            message.append::<Nonce>(&self.state.nonce);
            message.flush(Some(&self.state.digest))?;

            self.operationer.send().await?;
        }

        let local_addr = self.operationer.local_addr()?;
        let message = self.operationer.read_message().await?;

        ensure!(message.method() == ALLOCATE_RESPONSE);
        message.integrity(&self.state.digest)?;

        let relay = message.get::<XorRelayedAddress>().unwrap();

        ensure!(relay.ip() == self.server.ip());
        ensure!(message.get::<XorMappedAddress>() == Some(local_addr));
        ensure!(message.get::<Lifetime>() == Some(600));

        Ok(relay.port())
    }

    pub async fn create_permission(&mut self, port: u16) -> Result<()> {
        {
            let mut peer = self.server.clone();
            peer.set_port(port);

            let mut message = self.operationer.create_message(CREATE_PERMISSION_REQUEST);
            message.append::<XorPeerAddress>(peer);
            message.append::<UserName>(&self.credentials.username);
            message.append::<Realm>(&self.state.realm);
            message.append::<Nonce>(&self.state.nonce);
            message.flush(Some(&self.state.digest))?;

            self.operationer.send().await?;
        }

        let message = self.operationer.read_message().await?;

        ensure!(message.method() == CREATE_PERMISSION_RESPONSE);
        message.integrity(&self.state.digest)?;

        Ok(())
    }

    pub async fn channel_bind(&mut self, port: u16, channel: u16) -> Result<()> {
        {
            let mut peer = self.server.clone();
            peer.set_port(port);

            let mut message = self.operationer.create_message(CHANNEL_BIND_REQUEST);
            message.append::<ChannelNumber>(channel);
            message.append::<XorPeerAddress>(peer);
            message.append::<UserName>(&self.credentials.username);
            message.append::<Realm>(&self.state.realm);
            message.append::<Nonce>(&self.state.nonce);
            message.flush(Some(&self.state.digest))?;

            self.operationer.send().await?;
        }

        let message = self.operationer.read_message().await?;

        ensure!(message.method() == CHANNEL_BIND_RESPONSE);
        message.integrity(&self.state.digest)?;

        Ok(())
    }

    pub async fn refresh(&mut self, lifetime: u32) -> Result<()> {
        {
            let mut message = self.operationer.create_message(REFRESH_REQUEST);
            message.append::<Lifetime>(lifetime);
            message.append::<UserName>(&self.credentials.username);
            message.append::<Realm>(&self.state.realm);
            message.append::<Nonce>(&self.state.nonce);
            message.flush(Some(&self.state.digest))?;

            self.operationer.send().await?;
        }

        let message = self.operationer.read_message().await?;

        ensure!(message.method() == REFRESH_RESPONSE);
        message.integrity(&self.state.digest)?;

        ensure!(message.get::<Lifetime>() == Some(lifetime));

        Ok(())
    }

    pub async fn send_indication(&mut self, port: u16, data: &[u8]) -> Result<()> {
        let mut peer = self.server.clone();
        peer.set_port(port);

        let mut message = self.operationer.create_message(SEND_INDICATION);
        message.append::<XorPeerAddress>(peer);
        message.append::<Data>(data);
        message.flush(None)?;

        self.operationer.send().await?;
        Ok(())
    }

    pub async fn recv_indication(&mut self) -> Result<(u16, &[u8])> {
        let message = self.operationer.read_message().await?;

        ensure!(message.method() == DATA_INDICATION);

        let peer = message.get::<XorPeerAddress>().unwrap();
        let data = message.get::<Data>().unwrap();
        Ok((peer.port(), data))
    }

    pub async fn send_channel_data(&mut self, channel: u16, data: &[u8]) -> Result<()> {
        self.operationer.create_channel_data(channel, data);
        self.operationer.send().await?;
        Ok(())
    }

    pub async fn recv_channel_data(&mut self) -> Result<(u16, &[u8])> {
        let message = self.operationer.read_channel_data().await?;
        Ok((message.number, message.bytes))
    }
}

fn encode_password(username: &str, password: &str) -> Result<String> {
    Ok(BASE64_STANDARD.encode(
        hmac_sha1(password.as_bytes(), &[username.as_bytes()])?
            .into_bytes()
            .as_slice(),
    ))
}

#[tokio::test]
async fn integration_testing() -> Result<()> {
    create_turn_server(
        "127.0.0.1:3478".parse()?,
        Auth {
            static_auth_secret: Some("static_auth_secret".to_string()),
            static_credentials: {
                let mut it = HashMap::with_capacity(1);
                it.insert("static_credentials".to_string(), "static_credentials".to_string());
                it
            },
        },
        Api::default(),
    )
    .await?;

    let mut turn_1 = TurnClient::new(
        "127.0.0.1:3478".parse()?,
        Credentials {
            username: "static_credentials".to_string(),
            password: "static_credentials".to_string(),
        },
    )
    .await?;

    let mut turn_2 = TurnClient::new(
        "127.0.0.1:3478".parse()?,
        Credentials {
            username: "static_credentials".to_string(),
            password: "static_credentials".to_string(),
        },
    )
    .await?;

    let mut turn_3 = TurnClient::new(
        "127.0.0.1:3478".parse()?,
        Credentials {
            username: "static_auth_secret".to_string(),
            password: encode_password("static_auth_secret", "static_auth_secret")?,
        },
    )
    .await?;

    let mut turn_4 = TurnClient::new(
        "127.0.0.1:3478".parse()?,
        Credentials {
            username: "static_auth_secret".to_string(),
            password: encode_password("static_auth_secret", "static_auth_secret")?,
        },
    )
    .await?;

    {
        turn_1.binding().await?;
        turn_2.binding().await?;
        turn_3.binding().await?;
    }

    let turn_1_port = turn_1.allocate().await?;
    let turn_2_port = turn_2.allocate().await?;
    let turn_3_port = turn_3.allocate().await?;
    let turn_4_port = turn_4.allocate().await?;

    assert_eq!(turn_1.allocate().await?, turn_1_port);
    assert_eq!(turn_2.allocate().await?, turn_2_port);
    assert_eq!(turn_3.allocate().await?, turn_3_port);
    assert_eq!(turn_4.allocate().await?, turn_4_port);

    {
        turn_1.create_permission(turn_2_port).await?;
        turn_1.create_permission(turn_3_port).await?;
        turn_1.create_permission(turn_4_port).await?;
        turn_1.channel_bind(turn_2_port, 0x4000).await?;
        turn_1.channel_bind(turn_3_port, 0x4001).await?;
        turn_1.channel_bind(turn_4_port, 0x4002).await?;
        turn_1.refresh(600).await?;

        turn_2.create_permission(turn_1_port).await?;
        turn_2.create_permission(turn_3_port).await?;
        turn_2.channel_bind(turn_1_port, 0x4000).await?;
        turn_2.channel_bind(turn_3_port, 0x4002).await?;
        turn_2.refresh(600).await?;

        turn_3.create_permission(turn_1_port).await?;
        turn_3.create_permission(turn_2_port).await?;
        turn_3.channel_bind(turn_1_port, 0x4001).await?;
        turn_3.channel_bind(turn_2_port, 0x4002).await?;
        turn_3.refresh(600).await?;

        turn_4.create_permission(turn_1_port).await?;
        turn_4.channel_bind(turn_1_port, 0x4002).await?;
        turn_4.refresh(600).await?;

        assert!(turn_1.channel_bind(turn_2_port, 0x4000).await.is_ok());
        assert!(turn_1.channel_bind(turn_3_port, 0x4001).await.is_ok());
        assert!(turn_1.channel_bind(turn_4_port, 0x4002).await.is_ok());
        assert!(turn_2.channel_bind(turn_1_port, 0x4000).await.is_ok());
        assert!(turn_2.channel_bind(turn_3_port, 0x4002).await.is_ok());
        assert!(turn_3.channel_bind(turn_1_port, 0x4001).await.is_ok());
        assert!(turn_3.channel_bind(turn_2_port, 0x4002).await.is_ok());
        assert!(turn_4.channel_bind(turn_1_port, 0x4002).await.is_ok());
    }

    {
        let data = "1 forwards to 2,3,4 channel data".as_bytes();
        turn_1.send_channel_data(0x4000, data).await?;
        let ret = turn_2.recv_channel_data().await?;
        assert_eq!(ret.0, 0x4000);
        assert_eq!(ret.1, data);

        turn_1.send_channel_data(0x4001, data).await?;
        let ret = turn_3.recv_channel_data().await?;
        assert_eq!(ret.0, 0x4001);
        assert_eq!(ret.1, data);

        turn_1.send_channel_data(0x4002, data).await?;
        let ret = turn_4.recv_channel_data().await?;
        assert_eq!(ret.0, 0x4002);
        assert_eq!(ret.1, data);
    }

    {
        let data = "2 forwards to 1,3 channel data".as_bytes();
        turn_2.send_channel_data(0x4000, data).await?;
        let ret = turn_1.recv_channel_data().await?;
        assert_eq!(ret.0, 0x4000);
        assert_eq!(ret.1, data);
        assert!(turn_3.recv_channel_data().await.is_err());
        assert!(turn_4.recv_channel_data().await.is_err());

        turn_2.send_channel_data(0x4002, data).await?;
        let ret = turn_3.recv_channel_data().await?;
        assert_eq!(ret.0, 0x4002);
        assert_eq!(ret.1, data);
        assert!(turn_1.recv_channel_data().await.is_err());
        assert!(turn_4.recv_channel_data().await.is_err());

        turn_2.send_channel_data(0x4001, data).await?;
        assert!(turn_1.recv_channel_data().await.is_err());
        assert!(turn_3.recv_channel_data().await.is_err());
        assert!(turn_4.recv_channel_data().await.is_err());
    }

    {
        let data = "3 forwards to 1,2 channel data".as_bytes();
        turn_3.send_channel_data(0x4001, data).await?;
        let ret = turn_1.recv_channel_data().await?;
        assert_eq!(ret.0, 0x4001);
        assert_eq!(ret.1, data);
        assert!(turn_2.recv_channel_data().await.is_err());
        assert!(turn_4.recv_channel_data().await.is_err());

        turn_3.send_channel_data(0x4002, data).await?;
        let ret = turn_2.recv_channel_data().await?;
        assert_eq!(ret.0, 0x4002);
        assert_eq!(ret.1, data);
        assert!(turn_1.recv_channel_data().await.is_err());
        assert!(turn_4.recv_channel_data().await.is_err());

        turn_3.send_channel_data(0x4000, data).await?;
        assert!(turn_2.recv_channel_data().await.is_err());
        assert!(turn_1.recv_channel_data().await.is_err());
        assert!(turn_4.recv_channel_data().await.is_err());
    }

    {
        let data = "4 forwards to 1 channel data".as_bytes();
        turn_4.send_channel_data(0x4002, data).await?;
        let ret = turn_1.recv_channel_data().await?;
        assert_eq!(ret.0, 0x4002);
        assert_eq!(ret.1, data);
        assert!(turn_2.recv_channel_data().await.is_err());
        assert!(turn_3.recv_channel_data().await.is_err());

        turn_4.send_channel_data(0x4000, data).await?;
        assert!(turn_1.recv_channel_data().await.is_err());
        assert!(turn_2.recv_channel_data().await.is_err());
        assert!(turn_3.recv_channel_data().await.is_err());

        turn_4.send_channel_data(0x4001, data).await?;
        assert!(turn_1.recv_channel_data().await.is_err());
        assert!(turn_2.recv_channel_data().await.is_err());
        assert!(turn_3.recv_channel_data().await.is_err());
    }

    {
        let data = "1 forwards to 2,3,4".as_bytes();
        turn_1.send_indication(turn_2_port, data).await?;
        let ret = turn_2.recv_indication().await?;
        assert_eq!(ret.0, turn_1_port);
        assert_eq!(ret.1, data);

        turn_1.send_indication(turn_3_port, data).await?;
        let ret = turn_3.recv_indication().await?;
        assert_eq!(ret.0, turn_1_port);
        assert_eq!(ret.1, data);

        turn_1.send_indication(turn_4_port, data).await?;
        let ret = turn_4.recv_indication().await?;
        assert_eq!(ret.0, turn_1_port);
        assert_eq!(ret.1, data);
    }

    {
        let data = "2 forwards to 1,3".as_bytes();
        turn_2.send_indication(turn_1_port, data).await?;
        let ret = turn_1.recv_indication().await?;
        assert_eq!(ret.0, turn_2_port);
        assert_eq!(ret.1, data);

        turn_2.send_indication(turn_3_port, data).await?;
        let ret = turn_3.recv_indication().await?;
        assert_eq!(ret.0, turn_2_port);
        assert_eq!(ret.1, data);

        turn_2.send_indication(turn_4_port, data).await?;
        assert!(turn_4.recv_indication().await.is_err());
    }

    {
        let data = "3 forwards to 1,2".as_bytes();
        turn_3.send_indication(turn_1_port, data).await?;
        let ret = turn_1.recv_indication().await?;
        assert_eq!(ret.0, turn_3_port);
        assert_eq!(ret.1, data);

        turn_3.send_indication(turn_2_port, data).await?;
        let ret = turn_2.recv_indication().await?;
        assert_eq!(ret.0, turn_3_port);
        assert_eq!(ret.1, data);

        turn_3.send_indication(turn_4_port, data).await?;
        assert!(turn_4.recv_indication().await.is_err());
    }

    {
        let data = "4 forwards to 1".as_bytes();
        turn_4.send_indication(turn_1_port, data).await?;
        let ret = turn_1.recv_indication().await?;
        assert_eq!(ret.0, turn_4_port);
        assert_eq!(ret.1, data);

        turn_4.send_indication(turn_3_port, data).await?;
        assert!(turn_3.recv_indication().await.is_err());
    }

    {
        turn_1.refresh(0).await?;
        turn_2.refresh(0).await?;
        turn_3.refresh(0).await?;
    }

    Ok(())
}
