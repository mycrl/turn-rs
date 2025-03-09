#[cfg(test)]
mod tests {
    use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

    use anyhow::{ensure, Result};
    use async_trait::async_trait;
    use base64::{prelude::BASE64_STANDARD, Engine};
    use bytes::BytesMut;
    use stun::{
        attribute::{
            ChannelNumber, Data, ErrorCode, ErrorKind, Lifetime, MappedAddress, Nonce, Realm,
            ReqeestedTransport, ResponseOrigin, Transport, UserName, XorMappedAddress,
            XorPeerAddress, XorRelayedAddress,
        },
        ChannelData, Decoder, Kind, MessageReader, MessageWriter, Method, Payload,
    };
    use turn_driver::{
        start_hooks_server, Controller, Events, Hooks, SessionAddr, Transport as DriverTransport,
    };

    use once_cell::sync::Lazy;
    use rand::seq::SliceRandom;
    use tokio::{
        net::UdpSocket,
        time::{sleep, timeout},
    };

    use turn_server::{
        config::{Api, Auth, Config, Interface, Log, Transport as TurnTransport, Turn},
        startup,
    };

    static TOKEN: Lazy<[u8; 12]> = Lazy::new(|| {
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

        fn create_message(&mut self, method: Method) -> MessageWriter {
            MessageWriter::new(method, &TOKEN, &mut self.send_bytes)
        }

        fn create_channel_data(&mut self, number: u16, bytes: &[u8]) {
            ChannelData { number, bytes }.encode(&mut self.send_bytes);
        }

        async fn send(&self) -> Result<()> {
            self.socket.send(&self.send_bytes).await?;
            Ok(())
        }

        async fn read_message(&mut self) -> Result<MessageReader> {
            let size = timeout(
                Duration::from_secs(1),
                self.socket.recv(&mut self.recv_bytes),
            )
            .await??;

            if let Payload::Message(message) = self.decoder.decode(&self.recv_bytes[..size])? {
                if message.token != TOKEN.as_slice() {
                    Err(anyhow::anyhow!("Message token does not match"))
                } else {
                    Ok(message)
                }
            } else {
                Err(anyhow::anyhow!("payload not a message"))
            }
        }

        async fn read_channel_data(&mut self) -> Result<ChannelData> {
            let size = timeout(
                Duration::from_secs(1),
                self.socket.recv(&mut self.recv_bytes),
            )
            .await??;

            if let Payload::ChannelData(channel_data) =
                self.decoder.decode(&self.recv_bytes[..size])?
            {
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

        pub fn local_addr(&self) -> Result<SocketAddr> {
            Ok(self.operationer.local_addr()?)
        }

        pub async fn binding(&mut self) -> Result<()> {
            {
                let mut message = self
                    .operationer
                    .create_message(Method::Binding(Kind::Request));
                message.flush(None)?;

                self.operationer.send().await?;
            }

            let local_addr = self.operationer.local_addr()?;
            let message = self.operationer.read_message().await?;

            ensure!(message.method == Method::Binding(Kind::Response));
            ensure!(message.get::<XorMappedAddress>() == Some(local_addr));
            ensure!(message.get::<MappedAddress>() == Some(local_addr));
            ensure!(message.get::<ResponseOrigin>() == Some(self.server));
            Ok(())
        }

        pub async fn allocate(&mut self) -> Result<u16> {
            {
                {
                    let mut message = self
                        .operationer
                        .create_message(Method::Allocate(Kind::Request));
                    message.append::<ReqeestedTransport>(Transport::UDP);
                    message.flush(None)?;

                    self.operationer.send().await?;
                }

                let message = self.operationer.read_message().await?;

                ensure!(message.method == Method::Allocate(Kind::Error));
                ensure!(message.get::<ErrorCode>().unwrap().code == ErrorKind::Unauthorized as u16);

                self.state.nonce = message.get::<Nonce>().unwrap().to_string();
                self.state.realm = message.get::<Realm>().unwrap().to_string();
                self.state.digest = stun::util::long_term_credential_digest(
                    &self.credentials.username,
                    &self.credentials.password,
                    &self.state.realm,
                );
            }

            {
                let mut message = self
                    .operationer
                    .create_message(Method::Allocate(Kind::Request));
                message.append::<ReqeestedTransport>(Transport::UDP);
                message.append::<UserName>(&self.credentials.username);
                message.append::<Realm>(&self.state.realm);
                message.append::<Nonce>(&self.state.nonce);
                message.flush(Some(&self.state.digest))?;

                self.operationer.send().await?;
            }

            let local_addr = self.operationer.local_addr()?;
            let message = self.operationer.read_message().await?;

            ensure!(message.method == Method::Allocate(Kind::Response));
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

                let mut message = self
                    .operationer
                    .create_message(Method::CreatePermission(Kind::Request));
                message.append::<XorPeerAddress>(peer);
                message.append::<UserName>(&self.credentials.username);
                message.append::<Realm>(&self.state.realm);
                message.append::<Nonce>(&self.state.nonce);
                message.flush(Some(&self.state.digest))?;

                self.operationer.send().await?;
            }

            let message = self.operationer.read_message().await?;

            ensure!(message.method == Method::CreatePermission(Kind::Response));
            message.integrity(&self.state.digest)?;

            Ok(())
        }

        pub async fn channel_bind(&mut self, port: u16, channel: u16) -> Result<()> {
            {
                let mut peer = self.server.clone();
                peer.set_port(port);

                let mut message = self
                    .operationer
                    .create_message(Method::ChannelBind(Kind::Request));
                message.append::<ChannelNumber>(channel);
                message.append::<XorPeerAddress>(peer);
                message.append::<UserName>(&self.credentials.username);
                message.append::<Realm>(&self.state.realm);
                message.append::<Nonce>(&self.state.nonce);
                message.flush(Some(&self.state.digest))?;

                self.operationer.send().await?;
            }

            let message = self.operationer.read_message().await?;

            ensure!(message.method == Method::ChannelBind(Kind::Response));
            message.integrity(&self.state.digest)?;

            Ok(())
        }

        pub async fn refresh(&mut self, lifetime: u32) -> Result<()> {
            {
                let mut message = self
                    .operationer
                    .create_message(Method::Refresh(Kind::Request));
                message.append::<Lifetime>(lifetime);
                message.append::<UserName>(&self.credentials.username);
                message.append::<Realm>(&self.state.realm);
                message.append::<Nonce>(&self.state.nonce);
                message.flush(Some(&self.state.digest))?;

                self.operationer.send().await?;
            }

            let message = self.operationer.read_message().await?;

            ensure!(message.method == Method::Refresh(Kind::Response));
            message.integrity(&self.state.digest)?;

            ensure!(message.get::<Lifetime>() == Some(lifetime));

            Ok(())
        }

        pub async fn send_indication(&mut self, port: u16, data: &[u8]) -> Result<()> {
            let mut peer = self.server.clone();
            peer.set_port(port);

            let mut message = self.operationer.create_message(Method::SendIndication);
            message.append::<XorPeerAddress>(peer);
            message.append::<Data>(data);
            message.flush(None)?;

            self.operationer.send().await?;
            Ok(())
        }

        pub async fn recv_indication(&mut self) -> Result<(u16, &[u8])> {
            let message = self.operationer.read_message().await?;

            ensure!(message.method == Method::DataIndication);

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
            stun::util::hmac_sha1(password.as_bytes(), &[username.as_bytes()])?
                .into_bytes()
                .as_slice(),
        ))
    }

    struct HooksImpl(Arc<Controller>);

    #[async_trait]
    impl Hooks for HooksImpl {
        async fn auth(
            &self,
            _addr: &SessionAddr,
            username: &str,
            _realm: &str,
            _nonce: &str,
        ) -> Option<&str> {
            if username == "hooks" {
                Some("hooks")
            } else {
                None
            }
        }

        async fn on(&self, event: &Events, realm: &str, nonce: &str) {
            let get_session = |socket, username: String| async move {
                let ret = self.0.get_session(socket).await.unwrap();
                assert_eq!(ret.realm, realm);
                assert_eq!(ret.nonce, nonce);

                let session = ret.payload;
                assert_eq!(session.username, username);
                session
            };

            match event {
                Events::Allocated {
                    session,
                    username,
                    port,
                } => {
                    let session = get_session(session, username.to_string()).await;
                    assert_eq!(session.port, Some(*port));
                }
                Events::CreatePermission {
                    session,
                    username,
                    ports,
                } => {
                    let session = get_session(session, username.to_string()).await;
                    for port in ports {
                        if !session.permissions.contains(port) {
                            panic!()
                        }
                    }
                }
                Events::ChannelBind {
                    session,
                    username,
                    channel,
                } => {
                    let session = get_session(session, username.to_string()).await;
                    assert!(session.channels.contains(channel));
                }
                Events::Refresh {
                    session,
                    username,
                    lifetime,
                } => {
                    let session = get_session(session, username.to_string()).await;
                    assert!(session.expires >= *lifetime && session.expires <= lifetime + 10);
                }
                Events::Closed { session, .. } => {
                    assert!(self.0.get_session(session).await.is_none());
                }
            }
        }
    }

    #[tokio::test]
    async fn turn_static_auth_secret_testing() -> Result<()> {
        create_turn_server(
            "127.0.0.1:3479".parse()?,
            Auth {
                static_auth_secret: Some("static_auth_secret".to_string()),
                static_credentials: HashMap::with_capacity(1),
            },
            Api {
                bind: "127.0.0.1:3001".parse()?,
                hooks: None,
            },
        )
        .await?;

        let mut turn = TurnClient::new(
            "127.0.0.1:3479".parse()?,
            Credentials {
                username: "static_auth_secret".to_string(),
                password: encode_password("static_auth_secret", "static_auth_secret")?,
            },
        )
        .await?;

        turn.allocate().await?;
        Ok(())
    }

    #[tokio::test]
    async fn turn_server_testing() -> Result<()> {
        let controller = Arc::new(Controller::new("http://127.0.0.1:3000")?);
        {
            tokio::spawn(start_hooks_server(
                "127.0.0.1:8088".parse()?,
                HooksImpl(controller.clone()),
            ));

            sleep(Duration::from_secs(3)).await;
        }

        create_turn_server(
            "127.0.0.1:3478".parse()?,
            Auth {
                static_auth_secret: None,
                static_credentials: {
                    let mut it = HashMap::with_capacity(1);
                    it.insert(
                        "static_credentials".to_string(),
                        "static_credentials".to_string(),
                    );
                    it
                },
            },
            {
                let mut api = Api::default();
                api.hooks = Some("http://127.0.0.1:8088".to_string());
                api
            },
        )
        .await?;

        {
            let info = controller.get_info().await.unwrap().payload;
            assert_eq!(info.port_allocated, 0);
            assert_eq!(info.port_capacity, 16383);

            let interface = info.interfaces.get(0).unwrap();
            assert_eq!(interface.bind, "127.0.0.1:3478".parse()?);
            assert_eq!(interface.external, "127.0.0.1:3478".parse()?);
            assert_eq!(interface.transport, DriverTransport::UDP);
        }

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
                username: "hooks".to_string(),
                password: "hooks".to_string(),
            },
        )
        .await?;

        let mut turn_4 = TurnClient::new(
            "127.0.0.1:3478".parse()?,
            Credentials {
                username: "hooks".to_string(),
                password: "hooks".to_string(),
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
            let info = controller.get_info().await.unwrap().payload;
            assert_eq!(info.port_allocated, 4);
            assert_eq!(info.port_capacity, 16383);

            let interface = info.interfaces.get(0).unwrap();
            assert_eq!(interface.bind, "127.0.0.1:3478".parse()?);
            assert_eq!(interface.external, "127.0.0.1:3478".parse()?);
            assert_eq!(interface.transport, DriverTransport::UDP);
        }

        {
            turn_1.refresh(0).await?;
            turn_2.refresh(0).await?;
            turn_3.refresh(0).await?;
        }

        assert!(controller
            .get_session(&SessionAddr {
                address: turn_1.local_addr()?,
                interface: "127.0.0.1:3478".parse()?,
            })
            .await
            .is_none());

        assert!(controller
            .get_session(&SessionAddr {
                address: turn_2.local_addr()?,
                interface: "127.0.0.1:3478".parse()?,
            })
            .await
            .is_none());

        assert!(controller
            .get_session(&SessionAddr {
                address: turn_3.local_addr()?,
                interface: "127.0.0.1:3478".parse()?,
            })
            .await
            .is_none());

        assert!(controller
            .get_session(&SessionAddr {
                address: turn_4.local_addr()?,
                interface: "127.0.0.1:3478".parse()?,
            })
            .await
            .is_some());

        Ok(())
    }
}
