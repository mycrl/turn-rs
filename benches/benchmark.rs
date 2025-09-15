use std::{net::SocketAddr, sync::LazyLock};

use bytes::BytesMut;
use codec::{
    Decoder,
    channel_data::ChannelData,
    crypto::password_md5,
    message::{
        MessageEncoder,
        attributes::{ChannelNumber, Nonce, Realm, ReqeestedTransport, UserName, XorPeerAddress},
        methods::{
            ALLOCATE_REQUEST, BINDING_REQUEST, CHANNEL_BIND_REQUEST, CREATE_PERMISSION_REQUEST,
        },
    },
};

use criterion::*;
use rand::seq::SliceRandom;
use service::{
    Service, ServiceHandler, ServiceOptions,
    session::{Identifier, ports::PortRange},
};

#[derive(Clone)]
struct SimpleObserver;

impl ServiceHandler for SimpleObserver {
    fn get_password(&self, username: &str) -> Option<[u8; 16]> {
        Some(password_md5(username, "test", "test"))
    }
}

static TOKEN: LazyLock<[u8; 12]> = LazyLock::new(|| {
    let mut rng = rand::rng();
    let mut token = [0u8; 12];
    token.shuffle(&mut rng);
    token
});

fn criterion_benchmark(c: &mut Criterion) {
    let a_id = Identifier {
        source: "127.0.0.1:1000".parse().unwrap(),
        interface: "127.0.0.1:3478".parse().unwrap(),
    };

    let b_id = Identifier {
        source: "127.0.0.1:1001".parse().unwrap(),
        interface: "127.0.0.1:3478".parse().unwrap(),
    };

    let service = Service::new(ServiceOptions {
        realm: "test".to_string(),
        software: "test".to_string(),
        interfaces: vec![a_id.interface],
        handler: SimpleObserver,
        port_range: PortRange::default(),
    });

    let sessions = service.get_session_manager();
    let a_nonce = sessions
        .get_session(&a_id)
        .get_ref()
        .map(|it| it.nonce().clone())
        .unwrap();
    let a_integrity = sessions.get_password(&a_id, "test").unwrap();

    {
        let _ = sessions.get_password(&b_id, "test").unwrap();
    }

    let a_port = sessions.allocate(&a_id).unwrap();
    let b_port = sessions.allocate(&b_id).unwrap();

    {
        sessions.create_permission(&b_id, &b_id.source, &[a_port]);
        sessions.bind_channel(&b_id, &b_id.source, a_port, 0x4000);
    }

    let mut a_router = service.get_forwarder(a_id.source, a_id.interface);

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
        message.append::<ReqeestedTransport>(ReqeestedTransport::Udp);
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
                a_router.forward(&bind_request, a_id.source);
            })
        });

        turn.bench_function("allocate_request", |b| {
            b.iter(|| {
                a_router.forward(&allocate_request, a_id.source);
            })
        });

        turn.bench_function("create_permission_request", |b| {
            b.iter(|| {
                a_router.forward(&create_permission_request, a_id.source);
            })
        });

        turn.bench_function("channel_bind_request", |b| {
            b.iter(|| {
                a_router.forward(&channel_bind_request, a_id.source);
            })
        });

        turn.bench_function("channel_data", |b| {
            b.iter(|| {
                a_router.forward(&channel_data, a_id.source);
            })
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
