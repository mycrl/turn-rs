mod samples;

use std::net::{Ipv4Addr, SocketAddr};

use turn_server::{
    codec::{
        Decoder,
        crypto::{Password, generate_password},
        message::{attributes::*, methods::*},
    },
    service::{
        Service, ServiceHandler, ServiceOptions, routing::Response, session::ports::PortRange,
    },
};

#[derive(Default, Clone)]
struct AuthHandler;

impl AuthHandler {
    const fn username() -> &'static str {
        "user1"
    }

    const fn password() -> &'static str {
        "test"
    }

    const fn realm() -> &'static str {
        "localhost"
    }
}

impl ServiceHandler for AuthHandler {
    async fn get_password(&self, _: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
        Some(generate_password(
            Self::username(),
            Self::password(),
            Self::realm(),
            algorithm,
        ))
    }
}

#[tokio::test]
// #[rustfmt::skip]
async fn turn_test() {
    let mut decoder = Decoder::default();

    let interface = SocketAddr::from((Ipv4Addr::LOCALHOST, 3478));
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 51678));

    let service = Service::new(ServiceOptions {
        port_range: PortRange::default(),
        realm: AuthHandler::realm().to_string(),
        interfaces: vec![interface],
        handler: AuthHandler::default(),
    });

    let mut router = service.make_router(interface, interface);

    {
        let Response { bytes, .. } = router
            .route(samples::BINDING_REQUEST, addr)
            .await
            .unwrap()
            .unwrap();

        let message = decoder.decode(bytes).unwrap().into_message().unwrap();

        assert_eq!(message.method(), BINDING_RESPONSE);
        assert_eq!(message.get::<XorMappedAddress>(), Some(addr));
        assert_eq!(message.get::<MappedAddress>(), Some(addr));
        assert_eq!(message.get::<ResponseOrigin>(), Some(addr));
    }

    {
        let Response { bytes, .. } = router
            .route(samples::UNAUTHORIZED_ALLOCATE_REQUEST, addr)
            .await
            .unwrap()
            .unwrap();

        let message = decoder.decode(bytes).unwrap().into_message().unwrap();

        assert_eq!(message.method(), ALLOCATE_ERROR);
        assert_eq!(message.get::<ErrorCode>(), Some(ErrorCode::from(error::ErrorType::Unauthorized)));
        assert_eq!(message.get::<Realm>(), Some("localhost"));
        assert!(message.get::<Nonce>().is_some());
        assert!(message.get::<PasswordAlgorithms>().is_some());
    }

    {
        let Response { bytes, .. } = router
            .route(samples::ALLOCATE_REQUEST, addr)
            .await
            .unwrap()
            .unwrap();

        let message = decoder.decode(bytes).unwrap().into_message().unwrap();

        assert_eq!(message.method(), ALLOCATE_RESPONSE);
        assert_eq!(message.get::<XorMappedAddress>(), Some(addr));
        assert_eq!(message.get::<Lifetime>(), Some(600));

        let relayed = message.get::<XorRelayedAddress>().unwrap();
        assert_eq!(relayed.ip().to_string(), "127.0.0.1");
        assert!(relayed.port() >= 49152);
    }

    {
        let Response { bytes, .. } = router
            .route(samples::CREATE_PERMISSION_REQUEST, addr)
            .await
            .unwrap()
            .unwrap();

        let message = decoder.decode(bytes).unwrap().into_message().unwrap();

        assert_eq!(message.method(), CREATE_PERMISSION_RESPONSE);
        assert_eq!(message.get::<Lifetime>(), Some(600));
    }

    {
        let Response { bytes, .. } = router
            .route(samples::CHANNEL_BIND_REQUEST, addr)
            .await
            .unwrap()
            .unwrap();

        let message = decoder.decode(bytes).unwrap().into_message().unwrap();

        assert_eq!(message.method(), CHANNEL_BIND_RESPONSE);
    }

    {
        let Response { bytes, target, .. } = router
            .route(samples::SEND_INDICATION, addr)
            .await
            .unwrap()
            .unwrap();

        let message = decoder.decode(bytes).unwrap().into_message().unwrap();

        assert_eq!(message.method(), DATA_INDICATION);
        assert!(message.get::<XorPeerAddress>().is_some());
        assert!(message.get::<Data>().is_some());
        assert!(target.relay.is_some());
    }

    {
        let Response { bytes, .. } = router
            .route(samples::REFRESH_REQUEST, addr)
            .await
            .unwrap()
            .unwrap();

        let message = decoder.decode(bytes).unwrap().into_message().unwrap();

        assert_eq!(message.method(), REFRESH_RESPONSE);
        assert_eq!(message.get::<Lifetime>(), Some(600));
    }
}
