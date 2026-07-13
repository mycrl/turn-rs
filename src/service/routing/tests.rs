use std::net::SocketAddr;

use bytes::BytesMut;

use super::super::{InterfaceAddr, Service, ServiceHandler, ServiceOptions, Transport};
use crate::{
    codec::{
        Attributes,
        crypto::{Password, generate_password},
        message::{
            MessageEncoder,
            attributes::{ChannelNumber, PasswordAlgorithm, UserName, XorPeerAddress},
            methods::{CHANNEL_BIND_REQUEST, CREATE_PERMISSION_REQUEST},
        },
    },
    service::session::{Identifier, ports::PortRange},
};

const REALM: &str = "test-realm";
const USERNAME: &str = "test-user";
const PASSWORD: &str = "test-password";

#[derive(Clone)]
struct DenyPeerHandler;

impl ServiceHandler for DenyPeerHandler {
    async fn get_password(
        &self,
        _id: &Identifier,
        username: &str,
        algorithm: PasswordAlgorithm,
    ) -> Option<Password> {
        (username == USERNAME).then(|| generate_password(username, PASSWORD, REALM, algorithm))
    }

    fn allows_peer(&self, _client: &Identifier, _peer: SocketAddr) -> bool {
        false
    }
}

fn identifier(source: &str) -> Identifier {
    Identifier {
        source: source.parse().expect("test source address should parse"),
        external: "127.0.0.1:3478"
            .parse()
            .expect("test external address should parse"),
        interface: "127.0.0.1:3478"
            .parse()
            .expect("test interface address should parse"),
        transport: Transport::Udp,
    }
}

fn service() -> Service<DenyPeerHandler> {
    let client = identifier("127.0.0.1:50000");
    Service::new(ServiceOptions {
        port_range: PortRange::try_from(40000..40100).expect("test port range should be valid"),
        realm: REALM.to_string(),
        interfaces: vec![InterfaceAddr {
            addr: client.interface,
            external: client.external,
            transport: Transport::Udp,
        }],
        handler: DenyPeerHandler,
    })
}

async fn authenticate_and_allocate(
    service: &Service<DenyPeerHandler>,
    client: Identifier,
    peer: Identifier,
) -> u16 {
    service
        .get_session_manager()
        .get_password(&client, USERNAME, PasswordAlgorithm::Md5)
        .await
        .expect("client should authenticate");
    service
        .get_session_manager()
        .get_password(&peer, USERNAME, PasswordAlgorithm::Md5)
        .await
        .expect("peer should authenticate");
    service
        .get_session_manager()
        .allocate(&client, None)
        .expect("client should allocate a relay port");
    service
        .get_session_manager()
        .allocate(&peer, None)
        .expect("peer should allocate a relay port")
}

#[tokio::test]
async fn denied_peer_permission_returns_forbidden() {
    let client = identifier("127.0.0.1:50000");
    let peer = identifier("127.0.0.1:50001");
    let service = service();
    let password = generate_password(USERNAME, PASSWORD, REALM, PasswordAlgorithm::Md5);
    let peer_port = authenticate_and_allocate(&service, client, peer).await;

    let mut request = BytesMut::new();
    let mut encoder = MessageEncoder::new(CREATE_PERMISSION_REQUEST, &[7; 12], &mut request);
    encoder.append::<UserName>(USERNAME);
    encoder.append::<XorPeerAddress>(SocketAddr::new(peer.external.ip(), peer_port));
    encoder
        .flush(Some(&password))
        .expect("request should encode");

    let mut router = service.make_router(client);
    let mut response = BytesMut::new();
    let result = router
        .route(&request, &mut response)
        .await
        .expect("request should route")
        .expect("request should receive a response");

    assert_eq!(
        result.method,
        Some(crate::codec::message::methods::CREATE_PERMISSION_ERROR)
    );

    let mut attributes = Attributes::default();
    let response = crate::codec::message::Message::decode(&response, &mut attributes)
        .expect("response should decode");
    assert_eq!(
        response.method(),
        crate::codec::message::methods::CREATE_PERMISSION_ERROR
    );
}

#[tokio::test]
async fn denied_peer_channel_bind_returns_forbidden() {
    let client = identifier("127.0.0.1:50000");
    let peer = identifier("127.0.0.1:50001");
    let service = service();
    let password = generate_password(USERNAME, PASSWORD, REALM, PasswordAlgorithm::Md5);
    let peer_port = authenticate_and_allocate(&service, client, peer).await;

    let mut request = BytesMut::new();
    let mut encoder = MessageEncoder::new(CHANNEL_BIND_REQUEST, &[8; 12], &mut request);
    encoder.append::<UserName>(USERNAME);
    encoder.append::<XorPeerAddress>(SocketAddr::new(peer.external.ip(), peer_port));
    encoder.append::<ChannelNumber>(0x4000);
    encoder
        .flush(Some(&password))
        .expect("request should encode");

    let mut router = service.make_router(client);
    let mut response = BytesMut::new();
    let result = router
        .route(&request, &mut response)
        .await
        .expect("request should route")
        .expect("request should receive a response");

    assert_eq!(
        result.method,
        Some(crate::codec::message::methods::CHANNEL_BIND_ERROR)
    );
}
