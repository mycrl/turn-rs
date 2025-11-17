mod samples;

use std::sync::LazyLock;

use bytes::BytesMut;
use proptest::{array, char::range, collection::vec, option, prelude::*, prop_oneof};
use turn_server::prelude::*;

macro_rules! assert_decoded {
    (message {
        sample: $sample:expr,
        method: $method:expr
        $(, transaction_id: $txid:expr)?
        $(, attributes: [$($attr:ty => $expected:expr),* $(,)?])?
        $(, verify_with: $password:expr)?
        $(, assert: |$extra_msg:ident| $extra:block)?
        $(,)?
    }) => {{
        let mut decoder = Decoder::default();

        let message = decoder
            .decode($sample)
            .unwrap()
            .into_message()
            .unwrap();

        assert_eq!(message.method(), $method, "unexpected STUN method");

        $(
            assert_eq!(
                message.transaction_id(),
                $txid,
                "unexpected transaction id"
            );
        )?

        $(
            $(
                assert_eq!(
                    message.get::<$attr>(),
                    $expected,
                    "unexpected value for {}",
                    stringify!($attr)
                );
            )*
        )?

        $(
            message.verify($password).unwrap();
        )?

        $(
            {
                let $extra_msg = &message;
                $extra
            }
        )?
    }};
    (channel {
        sample: $sample:expr,
        number: $number:expr,
        bytes: $bytes:expr
        $(, assert: |$extra_data:ident| $extra:block)?
        $(,)?
    }) => {{
        let mut decoder = Decoder::default();

        let channel_data = decoder
            .decode($sample)
            .unwrap()
            .into_channel_data()
            .unwrap();

        assert_eq!(channel_data.number(), $number, "unexpected channel number");
        assert_eq!(channel_data.bytes(), $bytes, "unexpected channel payload");

        $(
            {
                let $extra_data = &channel_data;
                $extra
            }
        )?
    }};
}

static PASSWORD: LazyLock<Password> =
    LazyLock::new(|| generate_password("user1", "test", "localhost", PasswordAlgorithm::Md5));

#[test]
fn stun_binding_request() {
    assert_decoded!(message {
        sample: samples::BINDING_REQUEST,
        method: BINDING_REQUEST,
        transaction_id: &[
            0x45, 0x58, 0x65, 0x61, 0x57, 0x53, 0x5a, 0x6e, 0x57, 0x35, 0x76, 0x46
        ],
    });
}

#[test]
fn stun_binding_response() {
    assert_decoded!(message {
        sample: samples::BINDING_RESPONSE,
        method: BINDING_RESPONSE,
        attributes: [
            XorMappedAddress => Some("127.0.0.1:51678".parse().unwrap()),
            MappedAddress => Some("127.0.0.1:51678".parse().unwrap()),
            ResponseOrigin => Some("127.0.0.1:3478".parse().unwrap()),
        ],
    });
}

#[test]
fn stun_unauthorized_allocate_request() {
    assert_decoded!(message {
        sample: samples::UNAUTHORIZED_ALLOCATE_REQUEST,
        method: ALLOCATE_REQUEST,
        attributes: [RequestedTransport => Some(RequestedTransport::Udp)],
    });
}

#[test]
fn stun_unauthorized_allocate_response() {
    assert_decoded!(message {
        sample: samples::UNAUTHORIZED_ALLOCATE_RESPONSE,
        method: ALLOCATE_ERROR,
        attributes: [
            ErrorCode => Some(ErrorCode::from(ErrorType::Unauthorized)),
            Realm => Some("localhost"),
            Nonce => Some("UHm1hiE0jm9r9rGS"),
            PasswordAlgorithms => Some(vec![PasswordAlgorithm::Md5, PasswordAlgorithm::Sha256]),
        ],
    });
}

#[test]
fn stun_allocate_request() {
    assert_decoded!(message {
        sample: samples::ALLOCATE_REQUEST,
        method: ALLOCATE_REQUEST,
        attributes: [
            RequestedTransport => Some(RequestedTransport::Udp),
            UserName => Some("user1"),
            Realm => Some("localhost"),
            Nonce => Some("UHm1hiE0jm9r9rGS"),
        ],
        verify_with: &*PASSWORD,
    });
}

#[test]
fn stun_allocate_response() {
    assert_decoded!(message {
        sample: samples::ALLOCATE_RESPONSE,
        method: ALLOCATE_RESPONSE,
        attributes: [
            XorRelayedAddress => Some("127.0.0.1:55616".parse().unwrap()),
            XorMappedAddress => Some("127.0.0.1:51678".parse().unwrap()),
            Lifetime => Some(600),
        ],
        verify_with: &*PASSWORD,
    });
}

#[test]
fn stun_create_permission_request() {
    assert_decoded!(message {
        sample: samples::CREATE_PERMISSION_REQUEST,
        method: CREATE_PERMISSION_REQUEST,
        attributes: [
            XorPeerAddress => Some("127.0.0.1:55616".parse().unwrap()),
            UserName => Some("user1"),
            Realm => Some("localhost"),
            Nonce => Some("9jLBcjff3xrKRAES"),
        ],
        verify_with: &*PASSWORD,
    });
}

#[test]
fn stun_create_permission_response() {
    assert_decoded!(message {
        sample: samples::CREATE_PERMISSION_RESPONSE,
        method: CREATE_PERMISSION_RESPONSE,
        verify_with: &*PASSWORD,
    });
}

#[test]
fn stun_channel_bind_request() {
    assert_decoded!(message {
        sample: samples::CHANNEL_BIND_REQUEST,
        method: CHANNEL_BIND_REQUEST,
        attributes: [
            ChannelNumber => Some(0x4000),
            XorPeerAddress => Some("127.0.0.1:55616".parse().unwrap()),
            UserName => Some("user1"),
            Realm => Some("localhost"),
            Nonce => Some("9jLBcjff3xrKRAES"),
        ],
        verify_with: &*PASSWORD,
    });
}

#[test]
fn stun_channel_bind_response() {
    assert_decoded!(message {
        sample: samples::CHANNEL_BIND_RESPONSE,
        method: CHANNEL_BIND_RESPONSE,
        verify_with: &*PASSWORD,
    });
}

#[test]
fn stun_data_indication() {
    assert_decoded!(message {
        sample: samples::DATA_INDICATION,
        method: DATA_INDICATION,
        attributes: [XorPeerAddress => Some("127.0.0.1:55616".parse().unwrap())],
        assert: |msg| {
            assert_eq!(
                msg.get::<Data>().map(|it| it.len()),
                Some(100),
                "unexpected application data length"
            );
        },
    });
}

#[test]
fn stun_send_indication() {
    assert_decoded!(message {
        sample: samples::SEND_INDICATION,
        method: SEND_INDICATION,
        attributes: [XorPeerAddress => Some("127.0.0.1:55616".parse().unwrap())],
        assert: |msg| {
            assert_eq!(
                msg.get::<Data>().map(|it| it.len()),
                Some(96),
                "unexpected application data length"
            );
        },
    });
}

#[test]
fn stun_refresh_request() {
    assert_decoded!(message {
        sample: samples::REFRESH_REQUEST,
        method: REFRESH_REQUEST,
        attributes: [
            Lifetime => Some(0),
            UserName => Some("user1"),
            Realm => Some("localhost"),
            Nonce => Some("UHm1hiE0jm9r9rGS"),
        ],
        verify_with: &*PASSWORD,
    });
}

#[test]
fn stun_refresh_response() {
    assert_decoded!(message {
        sample: samples::REFRESH_RESPONSE,
        method: REFRESH_RESPONSE,
        attributes: [Lifetime => Some(0)],
        verify_with: &*PASSWORD,
    });
}

#[test]
fn stun_channel_data() {
    assert_decoded!(channel {
        sample: &samples::CHANNEL_DATA,
        number: 0x4000,
        bytes: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    });
}

/// Attempts to decode arbitrary input data once and asserts basic invariants
/// on any successfully decoded STUN message or ChannelData frame.
///
/// This helper is shared by the coverage-guided fuzz target to ensure that
/// any violation of decoder expectations triggers a panic and becomes a
/// reproducible corpus entry.
fn decode_and_assert(data: &[u8]) {
    if data.len() < 4 {
        return;
    }

    let mut decoder = Decoder::default();
    let Ok(result) = decoder.decode(data) else {
        return;
    };

    match result {
        DecodeResult::Message(message) => {
            assert!(message_invariants(data, &message));
        }
        DecodeResult::ChannelData(channel) => {
            assert!(channel_invariants(data, &channel));
        }
    }
}

fn message_invariants(data: &[u8], message: &Message<'_>) -> bool {
    if message.transaction_id().len() != 12 {
        return false;
    }

    if let Ok(size) = Decoder::message_size(data, false) {
        if size > data.len() {
            return false;
        }
    }

    for attr in [
        AttributeType::UserName,
        AttributeType::Realm,
        AttributeType::Nonce,
        AttributeType::MessageIntegrity,
        AttributeType::MessageIntegritySha256,
        AttributeType::Fingerprint,
    ] {
        if let Some(raw) = message.get_for_type(attr) {
            let base = data.as_ptr() as usize;
            let extent = base + data.len();
            let start = raw.as_ptr() as usize;
            let end = start + raw.len();

            if start < base || end > extent {
                return false;
            }
        }
    }

    let has_integrity = message
        .get_for_type(AttributeType::MessageIntegrity)
        .is_some()
        || message
            .get_for_type(AttributeType::MessageIntegritySha256)
            .is_some();

    if has_integrity {
        let test_password = Password::Md5([0u8; 16]);
        let verify_result = message.verify(&test_password);

        if !matches!(verify_result, Ok(()) | Err(Error::IntegrityFailed)) {
            return false;
        }
    }

    true
}

fn channel_invariants(data: &[u8], channel: &ChannelData<'_>) -> bool {
    let number = channel.number();
    if !(0x4000..0xFFFF).contains(&number) {
        return false;
    }

    let declared = u16::from_be_bytes([data[2], data[3]]) as usize + 4;
    declared <= data.len()
}

#[derive(Debug, Clone)]
struct GeneratedMessage {
    bytes: Vec<u8>,
    transaction_id: [u8; 12],
    username: Option<String>,
    realm: Option<String>,
    nonce: Option<String>,
    software: Option<String>,
    integrity_password: Option<Password>,
}

#[derive(Debug, Clone)]
struct GeneratedChannelData {
    bytes: Vec<u8>,
    number: u16,
    payload: Vec<u8>,
}

fn fuzz_inputs() -> impl Strategy<Value = Vec<u8>> {
    let raw = vec(any::<u8>(), 0..=2048);
    let valid_messages = valid_messages().prop_map(|msg| msg.bytes);
    let channel = valid_channel_data().prop_map(|cd| cd.bytes);

    prop_oneof![raw, valid_messages, channel]
}

fn text_token(max_len: usize) -> impl Strategy<Value = String> {
    vec(
        prop_oneof![
            Just('-'),
            Just('_'),
            Just('.'),
            Just(' '),
            range('0', '9'),
            range('a', 'z'),
            range('A', 'Z'),
        ],
        0..=max_len,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

fn maybe_text(max_len: usize) -> impl Strategy<Value = Option<String>> {
    option::of(text_token(max_len))
}

fn message_methods() -> impl Strategy<Value = Method> {
    prop_oneof![
        Just(BINDING_REQUEST),
        Just(ALLOCATE_REQUEST),
        Just(CREATE_PERMISSION_REQUEST),
        Just(CHANNEL_BIND_REQUEST),
        Just(REFRESH_REQUEST),
        Just(SEND_INDICATION),
        Just(DATA_INDICATION),
    ]
}

fn valid_messages() -> impl Strategy<Value = GeneratedMessage> {
    (
        message_methods(),
        array::uniform12(any::<u8>()),
        maybe_text(32),
        maybe_text(32),
        maybe_text(32),
        maybe_text(32),
        any::<bool>(),
    )
        .prop_map(
            |(method, transaction_id, username, realm, nonce, software, use_integrity)| {
                let mut buf = BytesMut::with_capacity(1024);
                let mut encoder = MessageEncoder::new(method, &transaction_id, &mut buf);

                if let Some(ref value) = username {
                    encoder.append::<UserName>(value);
                }

                if let Some(ref value) = realm {
                    encoder.append::<Realm>(value);
                }

                if let Some(ref value) = nonce {
                    encoder.append::<Nonce>(value);
                }

                if let Some(ref value) = software {
                    encoder.append::<Software>(value);
                }

                let password = use_integrity.then(|| Password::Md5([0u8; 16]));

                encoder
                    .flush(password.as_ref())
                    .expect("flushing generated message should succeed");

                GeneratedMessage {
                    bytes: buf.to_vec(),
                    transaction_id,
                    username,
                    realm,
                    nonce,
                    software,
                    integrity_password: password,
                }
            },
        )
}

fn valid_channel_data() -> impl Strategy<Value = GeneratedChannelData> {
    (0x4000u16..=0xFFFE, vec(any::<u8>(), 0..=512)).prop_map(|(number, payload)| {
        let mut buf = BytesMut::with_capacity(payload.len() + 4);
        ChannelData::new(number, &payload).encode(&mut buf);

        GeneratedChannelData {
            bytes: buf.to_vec(),
            number,
            payload,
        }
    })
}

proptest! {
    /// Mixed corpus containing arbitrary bytes, valid STUN messages, and valid ChannelData frames.
    /// Ensures decoder invariants hold without panicking across a wide input domain.
    #[test]
    fn decode_respects_invariants(data in fuzz_inputs()) {
        prop_assume!(data.len() >= 4);

        let mut decoder = Decoder::default();
        let result = decoder.decode(&data);
        prop_assume!(result.is_ok());
        let result = result.unwrap();

        match result {
            DecodeResult::Message(message) => {
                prop_assert!(message_invariants(&data, &message));
            }
            DecodeResult::ChannelData(channel) => {
                prop_assert!(channel_invariants(&data, &channel));
            }
        }
    }
}

proptest! {
    /// Valid STUN messages generated via MessageEncoder should decode back to the same view and preserve attributes.
    #[test]
    fn valid_messages_roundtrip(msg in valid_messages()) {
        let mut decoder = Decoder::default();
        let result = decoder.decode(&msg.bytes);

        prop_assert!(matches!(result, Ok(DecodeResult::Message(_))), "expected STUN message");
        let message = match result.unwrap() {
            DecodeResult::Message(message) => message,
            DecodeResult::ChannelData(_) => unreachable!(),
        };

        prop_assert_eq!(message.transaction_id(), msg.transaction_id.as_slice());

        let decoded_username = message.get::<UserName>().map(|s| s.to_owned());
        prop_assert_eq!(decoded_username.is_some(), msg.username.is_some(), "username presence mismatch");
        if let (Some(expected), Some(actual)) = (msg.username.as_ref(), decoded_username.as_ref()) {
            prop_assert_eq!(actual, expected);
        }

        let decoded_realm = message.get::<Realm>().map(|s| s.to_owned());
        prop_assert_eq!(decoded_realm.is_some(), msg.realm.is_some(), "realm presence mismatch");
        if let (Some(expected), Some(actual)) = (msg.realm.as_ref(), decoded_realm.as_ref()) {
            prop_assert_eq!(actual, expected);
        }

        let decoded_nonce = message.get::<Nonce>().map(|s| s.to_owned());
        prop_assert_eq!(decoded_nonce.is_some(), msg.nonce.is_some(), "nonce presence mismatch");
        if let (Some(expected), Some(actual)) = (msg.nonce.as_ref(), decoded_nonce.as_ref()) {
            prop_assert_eq!(actual, expected);
        }

        let decoded_software = message.get::<Software>().map(|s| s.to_owned());
        prop_assert_eq!(decoded_software.is_some(), msg.software.is_some(), "software presence mismatch");
        if let (Some(expected), Some(actual)) = (msg.software.as_ref(), decoded_software.as_ref()) {
            prop_assert_eq!(actual, expected);
        }

        if let Some(password) = msg.integrity_password {
            prop_assert!(message.verify(&password).is_ok());
        }
    }
}

proptest! {
    /// ChannelData frames generated via ChannelData::encode must round-trip through the decoder.
    #[test]
    fn channel_data_roundtrip(frame in valid_channel_data()) {
        let mut decoder = Decoder::default();
        let result = decoder.decode(&frame.bytes);
        prop_assert!(matches!(result, Ok(DecodeResult::ChannelData(_))), "expected ChannelData");
        let channel = match result.unwrap() {
            DecodeResult::ChannelData(channel) => channel,
            DecodeResult::Message(_) => unreachable!(),
        };

        prop_assert_eq!(channel.number(), frame.number);
        prop_assert_eq!(channel.bytes(), frame.payload.as_slice());
    }
}

#[test]
fn helper_returns_without_panicking_on_short_inputs() {
    for len in 0..4 {
        let data = vec![0u8; len];
        decode_and_assert(&data);
    }
}
