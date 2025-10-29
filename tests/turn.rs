use core::panic;
use std::net::{Ipv4Addr, SocketAddr};

use anyhow::Result;
use turn_server::{
    codec::{
        DecodeResult, Decoder,
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
fn test_turn_server_service() -> Result<()> {
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
            let response = router
                .route(include_bytes!("./samples/BindingRequest.bin"), client_addr)
                .await?;

            if let Some(Response::Message { method, bytes, .. }) = response {
                assert_eq!(method, BINDING_RESPONSE);

                if let DecodeResult::Message(message) = decoder.decode(bytes)? {
                    assert_eq!(message.method(), BINDING_RESPONSE);
                    assert_eq!(message.get::<XorMappedAddress>(), Some(client_addr));
                    assert_eq!(message.get::<MappedAddress>(), Some(client_addr));
                    assert_eq!(message.get::<ResponseOrigin>(), Some(client_addr));
                } else {
                    panic!("expected message");
                }
            } else {
                panic!("expected binding response");
            }
        }

        {
            let response = router
                .route(
                    include_bytes!("./samples/UnauthorizedAllocateRequest.bin"),
                    client_addr,
                )
                .await?;

            if let Some(Response::Message { method, bytes, .. }) = response {
                assert_eq!(method, ALLOCATE_ERROR);

                if let DecodeResult::Message(message) = decoder.decode(bytes)? {
                    assert_eq!(
                        message.get::<ErrorCode>(),
                        Some(ErrorCode::from(error::ErrorType::Unauthorized))
                    );
                    assert_eq!(message.get::<Realm>(), Some("localhost"));
                    assert!(message.get::<Nonce>().is_some());
                    assert!(message.get::<PasswordAlgorithms>().is_some());
                } else {
                    panic!("expected message");
                }
            } else {
                panic!("expected allocate error response");
            }
        }

        {
            let response = router
                .route(include_bytes!("./samples/AllocateRequest.bin"), client_addr)
                .await?;

            if let Some(Response::Message { method, bytes, .. }) = response {
                assert_eq!(method, ALLOCATE_RESPONSE);

                if let DecodeResult::Message(message) = decoder.decode(bytes)? {
                    let relayed = message.get::<XorRelayedAddress>().unwrap();
                    assert_eq!(relayed.ip().to_string(), "127.0.0.1");
                    assert!(relayed.port() >= 49152);
                    assert_eq!(message.get::<XorMappedAddress>(), Some(client_addr));
                    assert_eq!(message.get::<Lifetime>(), Some(600));
                } else {
                    panic!("expected message");
                }
            } else {
                panic!("expected allocate response");
            }
        }

        {
            let response = router
                .route(
                    include_bytes!("./samples/CreatePermissionRequest.bin"),
                    client_addr,
                )
                .await?;

            if let Some(Response::Message { method, bytes, .. }) = response {
                assert_eq!(method, CREATE_PERMISSION_RESPONSE);

                if let DecodeResult::Message(message) = decoder.decode(bytes)? {
                    assert_eq!(message.method(), CREATE_PERMISSION_RESPONSE);
                } else {
                    panic!("expected message");
                }
            } else {
                panic!("expected create permission response");
            }
        }

        {
            let response = router
                .route(
                    include_bytes!("./samples/ChannelBindRequest.bin"),
                    client_addr,
                )
                .await?;

            if let Some(Response::Message { method, bytes, .. }) = response {
                assert_eq!(method, CHANNEL_BIND_RESPONSE);

                if let DecodeResult::Message(message) = decoder.decode(bytes)? {
                    assert_eq!(message.method(), CHANNEL_BIND_RESPONSE);
                } else {
                    panic!("expected message");
                }
            } else {
                panic!("expected channel bind response");
            }
        }

        {
            let response = router
                .route(include_bytes!("./samples/SendIndication.bin"), client_addr)
                .await?;

            if let Some(Response::Message {
                method,
                bytes,
                target,
            }) = response
            {
                assert_eq!(method, DATA_INDICATION);

                if let DecodeResult::Message(message) = decoder.decode(bytes)? {
                    assert!(message.get::<XorPeerAddress>().is_some());
                    assert!(message.get::<Data>().is_some());
                    assert!(target.relay.is_some());
                } else {
                    panic!("expected message");
                }
            }
        }

        {
            let response = router
                .route(include_bytes!("./samples/RefreshRequest.bin"), client_addr)
                .await?;

            if let Some(Response::Message { method, bytes, .. }) = response {
                assert_eq!(method, REFRESH_RESPONSE);

                if let DecodeResult::Message(message) = decoder.decode(bytes)? {
                    assert_eq!(message.get::<Lifetime>(), Some(600));
                } else {
                    panic!("expected message");
                }
            } else {
                panic!("expected refresh response");
            }
        }

        Ok(())
    })
}
