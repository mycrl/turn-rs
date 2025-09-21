use anyhow::Result;
use turn_server_codec::{
    DecodeResult, Decoder,
    crypto::generate_password,
    message::{
        attributes::{error::ErrorType, *},
        methods::*,
    },
};

#[rustfmt::skip]
mod samples {
    pub const BINDING_REQUEST: &[u8] = include_bytes!("samples/BindingRequest.bin");
    pub const BINDING_RESPONSE: &[u8] = include_bytes!("samples/BindingResponse.bin");
    pub const UNAUTHORIZED_ALLOCATE_REQUEST: &[u8] = include_bytes!("samples/UnauthorizedAllocateRequest.bin");
    pub const UNAUTHORIZED_ALLOCATE_RESPONSE: &[u8] = include_bytes!("samples/UnauthorizedAllocateResponse.bin");
    pub const ALLOCATE_REQUEST: &[u8] = include_bytes!("samples/AllocateRequest.bin");
    pub const ALLOCATE_RESPONSE: &[u8] = include_bytes!("samples/AllocateResponse.bin");
    pub const CREATE_PERMISSION_REQUEST: &[u8] = include_bytes!("samples/CreatePermissionRequest.bin");
    pub const CREATE_PERMISSION_RESPONSE: &[u8] = include_bytes!("samples/CreatePermissionResponse.bin");
    pub const CHANNEL_BIND_REQUEST: &[u8] = include_bytes!("samples/ChannelBindRequest.bin");
    pub const CHANNEL_BIND_RESPONSE: &[u8] = include_bytes!("samples/ChannelBindResponse.bin");
    pub const DATA_INDICATION: &[u8] = include_bytes!("samples/DataIndication.bin");
    pub const SEND_INDICATION: &[u8] = include_bytes!("samples/SendIndication.bin");
    pub const REFRESH_REQUEST: &[u8] = include_bytes!("samples/RefreshRequest.bin");
    pub const REFRESH_RESPONSE: &[u8] = include_bytes!("samples/RefreshResponse.bin");
}

#[test]
#[rustfmt::skip]
fn test_turn_server_codec() -> Result<()> {
    let mut decoder = Decoder::default();

    {
        let message = decoder.decode(samples::BINDING_REQUEST)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), BINDING_REQUEST);
        assert_eq!(message.transaction_id(), &[0x45, 0x58, 0x65, 0x61, 0x57, 0x53, 0x5a, 0x6e, 0x57, 0x35, 0x76, 0x46]);
    }

    {
        let message = decoder.decode(samples::BINDING_RESPONSE)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), BINDING_RESPONSE);
        assert_eq!(message.get::<XorMappedAddress>(), Some("127.0.0.1:51678".parse()?));
        assert_eq!(message.get::<MappedAddress>(), Some("127.0.0.1:51678".parse()?));
        assert_eq!(message.get::<ResponseOrigin>(), Some("127.0.0.1:3478".parse()?));
    }

    {
        let message = decoder.decode(samples::UNAUTHORIZED_ALLOCATE_REQUEST)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), ALLOCATE_REQUEST);
        assert_eq!(message.get::<ReqeestedTransport>(), Some(ReqeestedTransport::Udp));
    }

    {
        let message = decoder.decode(samples::UNAUTHORIZED_ALLOCATE_RESPONSE)?;
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
        let message = decoder.decode(samples::ALLOCATE_REQUEST)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), ALLOCATE_REQUEST);
        assert_eq!(message.get::<ReqeestedTransport>(), Some(ReqeestedTransport::Udp));
        assert_eq!(message.get::<UserName>(), Some("user1"));
        assert_eq!(message.get::<Realm>(), Some("localhost"));
        assert_eq!(message.get::<Nonce>(), Some("UHm1hiE0jm9r9rGS"));

        message.checksum(&password)?;
    }

    {
        let message = decoder.decode(samples::ALLOCATE_RESPONSE)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), ALLOCATE_RESPONSE);
        assert_eq!(message.get::<XorRelayedAddress>(), Some("127.0.0.1:55616".parse()?));
        assert_eq!(message.get::<XorMappedAddress>(), Some("127.0.0.1:51678".parse()?));
        assert_eq!(message.get::<Lifetime>(), Some(600));

        message.checksum(&password)?;
    }

    {
        let message = decoder.decode(samples::CREATE_PERMISSION_REQUEST)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), CREATE_PERMISSION_REQUEST);
        assert_eq!(message.get::<XorPeerAddress>(), Some("127.0.0.1:55616".parse()?));
        assert_eq!(message.get::<UserName>(), Some("user1"));
        assert_eq!(message.get::<Realm>(), Some("localhost"));
        assert_eq!(message.get::<Nonce>(), Some("9jLBcjff3xrKRAES"));

        message.checksum(&password)?;
    }

    {
        let message = decoder.decode(samples::CREATE_PERMISSION_RESPONSE)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), CREATE_PERMISSION_RESPONSE);

        message.checksum(&password)?;
    }

    {
        let message = decoder.decode(samples::CHANNEL_BIND_REQUEST)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), CHANNEL_BIND_REQUEST);
        assert_eq!(message.get::<ChannelNumber>(), Some(0x4000));
        assert_eq!(message.get::<XorPeerAddress>(), Some("127.0.0.1:55616".parse()?));
        assert_eq!(message.get::<UserName>(), Some("user1"));
        assert_eq!(message.get::<Realm>(), Some("localhost"));
        assert_eq!(message.get::<Nonce>(), Some("9jLBcjff3xrKRAES"));

        message.checksum(&password)?;
    }

    {
        let message = decoder.decode(samples::CHANNEL_BIND_RESPONSE)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), CHANNEL_BIND_RESPONSE);

        message.checksum(&password)?;
    }

    {
        let message = decoder.decode(samples::DATA_INDICATION)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), DATA_INDICATION);
        assert_eq!(message.get::<XorPeerAddress>(), Some("127.0.0.1:55616".parse()?));
        assert_eq!(message.get::<Data>().map(|it| it.len()), Some(100));
    }

    {
        let message = decoder.decode(samples::SEND_INDICATION)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), SEND_INDICATION);
        assert_eq!(message.get::<XorPeerAddress>(), Some("127.0.0.1:55616".parse()?));
        assert_eq!(message.get::<Data>().map(|it| it.len()), Some(96));
    }

    {
        let message = decoder.decode(samples::REFRESH_REQUEST)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), REFRESH_REQUEST);
        assert_eq!(message.get::<Lifetime>(), Some(0));
        assert_eq!(message.get::<UserName>(), Some("user1"));
        assert_eq!(message.get::<Realm>(), Some("localhost"));
        assert_eq!(message.get::<Nonce>(), Some("UHm1hiE0jm9r9rGS"));

        message.checksum(&password)?;
    }

    {
        let message = decoder.decode(samples::REFRESH_RESPONSE)?;
        let DecodeResult::Message(message) = message else {
            return Err(anyhow::anyhow!("Expected Message"));
        };

        assert_eq!(message.method(), REFRESH_RESPONSE);
        assert_eq!(message.get::<Lifetime>(), Some(0));

        message.checksum(&password)?;
    }

    Ok(())
}
