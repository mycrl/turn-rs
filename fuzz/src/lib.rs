#![cfg(test)]

use bytes::BytesMut;
use proptest::{
    array,
    char::range,
    collection::vec,
    option,
    prelude::*,
    prop_oneof,
};

use turn_server::codec::{
    channel_data::ChannelData,
    crypto::Password,
    message::{
        MessageEncoder,
        attributes::{AttributeType, Nonce, Realm, Software, UserName},
        methods::{
            Method,
            ALLOCATE_REQUEST,
            CHANNEL_BIND_REQUEST,
            CREATE_PERMISSION_REQUEST,
            DATA_INDICATION,
            REFRESH_REQUEST,
            SEND_INDICATION,
            BINDING_REQUEST,
        },
    },
    DecodeResult, Decoder, Error,
};

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
    (message_methods(), array::uniform12(any::<u8>()), maybe_text(32), maybe_text(32), maybe_text(32), maybe_text(32), any::<bool>())
        .prop_map(|(method, transaction_id, username, realm, nonce, software, use_integrity)| {
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
        })
}

fn valid_channel_data() -> impl Strategy<Value = GeneratedChannelData> {
    (
        0x4000u16..=0xFFFE,
        vec(any::<u8>(), 0..=512),
    )
        .prop_map(|(number, payload)| {
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
                prop_assert_eq!(message.transaction_id().len(), 12);

                if let Ok(sz) = Decoder::message_size(&data, false) {
                    prop_assert!(sz <= data.len());
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

                        prop_assert!(start >= base && end <= extent);
                    }
                }

                let has_integrity = message.get_for_type(AttributeType::MessageIntegrity).is_some()
                    || message.get_for_type(AttributeType::MessageIntegritySha256).is_some();

                if has_integrity {
                    let test_password = Password::Md5([0u8; 16]);
                    let verify_result = message.verify(&test_password);

                    prop_assert!(matches!(verify_result, Ok(()) | Err(Error::IntegrityFailed)));
                }
            }
            DecodeResult::ChannelData(channel) => {
                let number = channel.number();
                prop_assert!((0x4000..0xFFFF).contains(&number));

                let declared = u16::from_be_bytes([data[2], data[3]]) as usize + 4;
                prop_assert!(declared <= data.len());
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

