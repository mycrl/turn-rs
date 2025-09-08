use std::{net::SocketAddr, sync::LazyLock};

use bytes::BytesMut;
use criterion::*;
use rand::seq::SliceRandom;
use turn_server::{
    stun::{
        ChannelData, Decoder, MessageEncoder, Transport,
        attribute::{ChannelNumber, Nonce, Realm, ReqeestedTransport, UserName, XorPeerAddress},
        method::{ALLOCATE_REQUEST, BINDING_REQUEST, CHANNEL_BIND_REQUEST, CREATE_PERMISSION_REQUEST},
    },
    turn::{Observer, Service, SessionAddr},
};

#[derive(Clone)]
struct SimpleObserver;

impl Observer for SimpleObserver {
    fn get_password(&self, _: &str) -> Option<String> {
        Some("test".to_string())
    }
}

static TOKEN: LazyLock<[u8; 12]> = LazyLock::new(|| {
    let mut rng = rand::rng();
    let mut token = [0u8; 12];
    token.shuffle(&mut rng);
    token
});

fn criterion_benchmark(c: &mut Criterion) {
    let a_session_addr = SessionAddr {
        address: "127.0.0.1:1000".parse().unwrap(),
        interface: "127.0.0.1:3478".parse().unwrap(),
    };

    let b_session_addr = SessionAddr {
        address: "127.0.0.1:1001".parse().unwrap(),
        interface: "127.0.0.1:3478".parse().unwrap(),
    };

    let service = Service::new(
        "test".to_string(),
        "test".to_string(),
        vec![a_session_addr.interface],
        SimpleObserver,
    );

    let sessions = service.get_sessions();
    let a_nonce = sessions
        .get_nonce(&a_session_addr)
        .get_ref()
        .map(|it| it.0.clone())
        .unwrap();
    let a_integrity = sessions.get_integrity(&a_session_addr, "test", "test").unwrap();

    {
        let _ = sessions.get_integrity(&b_session_addr, "test", "test").unwrap();
    }

    let a_port = sessions.allocate(&a_session_addr).unwrap();
    let b_port = sessions.allocate(&b_session_addr).unwrap();

    {
        sessions.create_permission(&b_session_addr, &b_session_addr.address, &[a_port]);
        sessions.bind_channel(&b_session_addr, &b_session_addr.address, a_port, 0x4000);
    }

    let mut a_operationer = service.get_operationer(a_session_addr.address, a_session_addr.interface);

    let bind_request = {
        let mut bytes = BytesMut::zeroed(1500);
        MessageEncoder::new(BINDING_REQUEST, &TOKEN, &mut bytes)
            .flush(None)
            .unwrap();
        bytes
    };

    let allocate_request = {
        let mut bytes = BytesMut::zeroed(1500);
        let mut message = MessageEncoder::new(ALLOCATE_REQUEST, &TOKEN, &mut bytes);
        message.append::<ReqeestedTransport>(Transport::UDP);
        message.append::<UserName>("test");
        message.append::<Realm>("test");
        message.append::<Nonce>(&a_nonce);
        message.flush(Some(&a_integrity)).unwrap();
        bytes
    };

    let create_permission_request = {
        let mut bytes = BytesMut::zeroed(1500);
        let mut message = MessageEncoder::new(CREATE_PERMISSION_REQUEST, &TOKEN, &mut bytes);
        message.append::<XorPeerAddress>(SocketAddr::new("127.0.0.1".parse().unwrap(), b_port));
        message.append::<UserName>("test");
        message.append::<Realm>("test");
        message.append::<Nonce>(&a_nonce);
        message.flush(Some(&a_integrity)).unwrap();
        bytes
    };

    let channel_bind_request = {
        let mut bytes = BytesMut::zeroed(1500);
        let mut message = MessageEncoder::new(CHANNEL_BIND_REQUEST, &TOKEN, &mut bytes);
        message.append::<ChannelNumber>(0x4000);
        message.append::<XorPeerAddress>(SocketAddr::new("127.0.0.1".parse().unwrap(), b_port));
        message.append::<UserName>("test");
        message.append::<Realm>("test");
        message.append::<Nonce>(&a_nonce);
        message.flush(Some(&a_integrity)).unwrap();
        bytes
    };

    let channel_data = {
        let mut bytes = BytesMut::zeroed(1500);
        ChannelData {
            number: 0x4000,
            bytes: TOKEN.as_slice(),
        }
        .encode(&mut bytes);
        bytes
    };

    {
        let mut stun = c.benchmark_group("stun");
        let mut codec = Decoder::default();

        stun.throughput(Throughput::Elements(1));

        stun.bench_function("decode_binding_request", |b| {
            b.iter(|| {
                codec.decode(&bind_request).unwrap();
            })
        });

        stun.bench_function("decode_allocate_request", |b| {
            b.iter(|| {
                codec.decode(&allocate_request).unwrap();
            })
        });

        stun.bench_function("decode_create_permission_request", |b| {
            b.iter(|| {
                codec.decode(&create_permission_request).unwrap();
            })
        });

        stun.bench_function("decode_channel_bind_request", |b| {
            b.iter(|| {
                codec.decode(&channel_bind_request).unwrap();
            })
        });

        stun.bench_function("decode_channel_data", |b| {
            b.iter(|| {
                codec.decode(&channel_data).unwrap();
            })
        });

        stun.finish();
    }

    {
        let mut turn = c.benchmark_group("turn");

        turn.throughput(Throughput::Elements(1));

        turn.bench_function("bind_request", |b| {
            b.iter(|| {
                a_operationer.route(&bind_request, a_session_addr.address).unwrap();
            })
        });

        turn.bench_function("allocate_request", |b| {
            b.iter(|| {
                a_operationer.route(&allocate_request, a_session_addr.address).unwrap();
            })
        });

        turn.bench_function("create_permission_request", |b| {
            b.iter(|| {
                a_operationer
                    .route(&create_permission_request, a_session_addr.address)
                    .unwrap();
            })
        });

        turn.bench_function("channel_bind_request", |b| {
            b.iter(|| {
                a_operationer
                    .route(&channel_bind_request, a_session_addr.address)
                    .unwrap();
            })
        });

        turn.bench_function("channel_data", |b| {
            b.iter(|| {
                a_operationer.route(&channel_data, a_session_addr.address).unwrap();
            })
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
