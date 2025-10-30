use std::net::{Ipv4Addr, SocketAddr};

use anyhow::Result;
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
    const fn password() -> &'static str {
        "test"
    }

    const fn realm() -> &'static str {
        "localhost"
    }
}

impl ServiceHandler for AuthHandler {
    async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
        Some(generate_password(
            username,
            Self::password(),
            Self::realm(),
            algorithm,
        ))
    }
}

#[test]
fn turn_test() -> Result<()> {
    pollster::block_on(async {
        let mut decoder = Decoder::default();

        let base_addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 3478));
        let client_addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 51678));

        let service = Service::new(ServiceOptions {
            port_range: PortRange::default(),
            realm: "localhost".to_string(),
            interfaces: vec![base_addr],
            handler: AuthHandler::default(),
        });

        let mut router = service.make_router(base_addr, base_addr);

        {
            let Response { bytes, .. } = router
                .route(include_bytes!("./samples/BindingRequest.bin"), client_addr)
                .await?
                .expect("expected binding response");

            let message = decoder.decode(bytes)?.into_message().expect("expected message");
            assert_eq!(message.method(), BINDING_RESPONSE);
            assert_eq!(message.get::<XorMappedAddress>(), Some(client_addr));
            assert_eq!(message.get::<MappedAddress>(), Some(client_addr));
            assert_eq!(message.get::<ResponseOrigin>(), Some(base_addr));
        }

        {
            let Response { method, bytes, .. } = router
                .route(
                    include_bytes!("./samples/UnauthorizedAllocateRequest.bin"),
                    client_addr,
                )
                .await?
                .expect("expected allocate error response");

            assert_eq!(method, Some(ALLOCATE_ERROR));

            let message = decoder.decode(bytes)?.into_message().expect("expected message");
            assert_eq!(
                message.get::<ErrorCode>(),
                Some(ErrorCode::from(error::ErrorType::Unauthorized))
            );
            assert_eq!(message.get::<Realm>(), Some("localhost"));
            assert!(message.get::<Nonce>().is_some());
            assert!(message.get::<PasswordAlgorithms>().is_some());
        }

        {
            let Response { method, bytes, .. } = router
                .route(include_bytes!("./samples/AllocateRequest.bin"), client_addr)
                .await?
                .expect("expected allocate response");

            assert_eq!(method, Some(ALLOCATE_RESPONSE));

            let message = decoder.decode(bytes)?.into_message().expect("expected message");
            let relayed = message.get::<XorRelayedAddress>().unwrap();
            assert_eq!(relayed.ip().to_string(), "127.0.0.1");
            assert!(relayed.port() >= 49152);
            assert_eq!(message.get::<XorMappedAddress>(), Some(client_addr));
            assert_eq!(message.get::<Lifetime>(), Some(600));
        }

        {
            let Response { method, bytes, .. } = router
                .route(
                    include_bytes!("./samples/CreatePermissionRequest.bin"),
                    client_addr,
                )
                .await?
                .expect("expected create permission response");

            assert_eq!(method, Some(CREATE_PERMISSION_RESPONSE));

            let message = decoder.decode(bytes)?.into_message().expect("expected message");
            assert_eq!(message.method(), CREATE_PERMISSION_RESPONSE);
        }

        {
            let Response { method, bytes, .. } = router
                .route(
                    include_bytes!("./samples/ChannelBindRequest.bin"),
                    client_addr,
                )
                .await?
                .expect("expected channel bind response");

            assert_eq!(method, Some(CHANNEL_BIND_RESPONSE));

            let message = decoder.decode(bytes)?.into_message().expect("expected message");
            assert_eq!(message.method(), CHANNEL_BIND_RESPONSE);
        }

        {
            let Response { method, bytes, target } = router
                .route(include_bytes!("./samples/SendIndication.bin"), client_addr)
                .await?
                .expect("expected send indication response");

            assert_eq!(method, Some(DATA_INDICATION));

            let message = decoder.decode(bytes)?.into_message().expect("expected message");
            assert!(message.get::<XorPeerAddress>().is_some());
            assert!(message.get::<Data>().is_some());
            assert!(target.relay.is_some());
        }

        {
            let Response { method, bytes, .. } = router
                .route(include_bytes!("./samples/RefreshRequest.bin"), client_addr)
                .await?
                .expect("expected refresh response");

            assert_eq!(method, Some(REFRESH_RESPONSE));

            let message = decoder.decode(bytes)?.into_message().expect("expected message");
            assert_eq!(message.get::<Lifetime>(), Some(600));
        }

        Ok(())
    })
}
