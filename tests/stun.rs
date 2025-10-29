use anyhow::Result;
use turn_server::codec::{
    DecodeResult, Decoder,
    crypto::generate_password,
    message::{
        attributes::{error::ErrorType, *},
        methods::*,
    },
};

#[test]
#[rustfmt::skip]
fn test_turn_server_codec() -> Result<()> {
    let mut decoder = Decoder::default();

    {
        let message = decoder.decode(include_bytes!("./samples/BindingRequest.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), BINDING_REQUEST);
        assert_eq!(message.transaction_id(), &[0x45, 0x58, 0x65, 0x61, 0x57, 0x53, 0x5a, 0x6e, 0x57, 0x35, 0x76, 0x46]);
    }

    {
        let message = decoder.decode(include_bytes!("./samples/BindingResponse.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), BINDING_RESPONSE);
        assert_eq!(message.get::<XorMappedAddress>(), Some("127.0.0.1:51678".parse()?));
        assert_eq!(message.get::<MappedAddress>(), Some("127.0.0.1:51678".parse()?));
        assert_eq!(message.get::<ResponseOrigin>(), Some("127.0.0.1:3478".parse()?));
    }

    {
        let message = decoder.decode(include_bytes!("./samples/UnauthorizedAllocateRequest.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), ALLOCATE_REQUEST);
        assert_eq!(message.get::<ReqeestedTransport>(), Some(ReqeestedTransport::Udp));
    }

    {
        let message = decoder.decode(include_bytes!("./samples/UnauthorizedAllocateResponse.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), ALLOCATE_ERROR);
        assert_eq!(message.get::<ErrorCode>(), Some(ErrorCode::from(ErrorType::Unauthorized)));
        assert_eq!(message.get::<Realm>(), Some("localhost"));
        assert_eq!(message.get::<Nonce>(), Some("UHm1hiE0jm9r9rGS"));
        assert_eq!(message.get::<PasswordAlgorithms>(), Some(vec![PasswordAlgorithm::Md5, PasswordAlgorithm::Sha256]));
    }

    let password = generate_password("user1", "test", "localhost", PasswordAlgorithm::Md5);

    {
        let message = decoder.decode(include_bytes!("./samples/AllocateRequest.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), ALLOCATE_REQUEST);
        assert_eq!(message.get::<ReqeestedTransport>(), Some(ReqeestedTransport::Udp));
        assert_eq!(message.get::<UserName>(), Some("user1"));
        assert_eq!(message.get::<Realm>(), Some("localhost"));
        assert_eq!(message.get::<Nonce>(), Some("UHm1hiE0jm9r9rGS"));

        message.verify(&password)?;
    }

    {
        let message = decoder.decode(include_bytes!("./samples/AllocateResponse.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), ALLOCATE_RESPONSE);
        assert_eq!(message.get::<XorRelayedAddress>(), Some("127.0.0.1:55616".parse()?));
        assert_eq!(message.get::<XorMappedAddress>(), Some("127.0.0.1:51678".parse()?));
        assert_eq!(message.get::<Lifetime>(), Some(600));

        message.verify(&password)?;
    }

    {
        let message = decoder.decode(include_bytes!("./samples/CreatePermissionRequest.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), CREATE_PERMISSION_REQUEST);
        assert_eq!(message.get::<XorPeerAddress>(), Some("127.0.0.1:55616".parse()?));
        assert_eq!(message.get::<UserName>(), Some("user1"));
        assert_eq!(message.get::<Realm>(), Some("localhost"));
        assert_eq!(message.get::<Nonce>(), Some("9jLBcjff3xrKRAES"));

        message.verify(&password)?;
    }

    {
        let message = decoder.decode(include_bytes!("./samples/CreatePermissionResponse.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), CREATE_PERMISSION_RESPONSE);

        message.verify(&password)?;
    }

    {
        let message = decoder.decode(include_bytes!("./samples/ChannelBindRequest.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), CHANNEL_BIND_REQUEST);
        assert_eq!(message.get::<ChannelNumber>(), Some(0x4000));
        assert_eq!(message.get::<XorPeerAddress>(), Some("127.0.0.1:55616".parse()?));
        assert_eq!(message.get::<UserName>(), Some("user1"));
        assert_eq!(message.get::<Realm>(), Some("localhost"));
        assert_eq!(message.get::<Nonce>(), Some("9jLBcjff3xrKRAES"));

        message.verify(&password)?;
    }

    {
        let message = decoder.decode(include_bytes!("./samples/ChannelBindResponse.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), CHANNEL_BIND_RESPONSE);

        message.verify(&password)?;
    }

    {
        let message = decoder.decode(include_bytes!("./samples/DataIndication.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), DATA_INDICATION);
        assert_eq!(message.get::<XorPeerAddress>(), Some("127.0.0.1:55616".parse()?));
        assert_eq!(message.get::<Data>().map(|it| it.len()), Some(100));
    }

    {
        let message = decoder.decode(include_bytes!("./samples/SendIndication.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), SEND_INDICATION);
        assert_eq!(message.get::<XorPeerAddress>(), Some("127.0.0.1:55616".parse()?));
        assert_eq!(message.get::<Data>().map(|it| it.len()), Some(96));
    }

    {
        let message = decoder.decode(include_bytes!("./samples/RefreshRequest.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), REFRESH_REQUEST);
        assert_eq!(message.get::<Lifetime>(), Some(0));
        assert_eq!(message.get::<UserName>(), Some("user1"));
        assert_eq!(message.get::<Realm>(), Some("localhost"));
        assert_eq!(message.get::<Nonce>(), Some("UHm1hiE0jm9r9rGS"));

        message.verify(&password)?;
    }

    {
        let message = decoder.decode(include_bytes!("./samples/RefreshResponse.bin"))?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), REFRESH_RESPONSE);
        assert_eq!(message.get::<Lifetime>(), Some(0));

        message.verify(&password)?;
    }

    Ok(())
}
