use std::sync::Arc;

use bytes::BytesMut;
use criterion::*;
use parking_lot::Mutex;
use rand::seq::SliceRandom;
use turn_server::prelude::*;

#[derive(Clone)]
struct DummyHandler;

impl ServiceHandler for DummyHandler {
    async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
        Some(generate_password(username, "test", "test", algorithm))
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let runtime_handle = runtime.handle();

    let local_id = Identifier {
        source: "127.0.0.1:1000".parse().unwrap(),
        interface: "127.0.0.1:3478".parse().unwrap(),
    };

    let remote_id = Identifier {
        source: "127.0.0.1:1001".parse().unwrap(),
        interface: "127.0.0.1:3478".parse().unwrap(),
    };

    let service = Service::new(ServiceOptions {
        port_range: Default::default(),
        realm: "test".to_string(),
        interfaces: vec!["127.0.0.1:3478".parse().unwrap()],
        handler: DummyHandler,
    });

    let session_manager = service.get_session_manager();

    session_manager.get_session_or_default(&local_id);
    session_manager.get_session_or_default(&remote_id);

    runtime_handle
        .block_on(session_manager.get_password(&local_id, "test", PasswordAlgorithm::Md5))
        .unwrap();

    runtime_handle
        .block_on(session_manager.get_password(&remote_id, "test", PasswordAlgorithm::Md5))
        .unwrap();

    let local_port = session_manager.allocate(&local_id, None).unwrap();
    let remote_port = session_manager.allocate(&remote_id, None).unwrap();

    session_manager.create_permission(&remote_id, &remote_id.source, &[local_port]);
    session_manager.create_permission(&local_id, &local_id.source, &[remote_port]);

    session_manager.bind_channel(&remote_id, &remote_id.source, local_port, 0x4000);
    session_manager.bind_channel(&local_id, &local_id.source, remote_port, 0x4000);

    let router = service.make_router(local_id.source, local_id.interface);

    let transaction_id = {
        let mut rng = rand::rng();
        let mut token = [0u8; 12];

        token.shuffle(&mut rng);
        token
    };

    let bind_request = {
        let mut bytes = BytesMut::zeroed(1500);

        MessageEncoder::new(BINDING_REQUEST, &transaction_id, &mut bytes)
            .flush(None)
            .unwrap();

        bytes
    };

    let indication = {
        let mut bytes = BytesMut::zeroed(1500);

        let mut encoder = MessageEncoder::new(SEND_INDICATION, &transaction_id, &mut bytes);
        encoder.append::<XorPeerAddress>(format!("127.0.0.1:{remote_port}").parse().unwrap());
        encoder.append::<Data>(&transaction_id);
        encoder.flush(None).unwrap();

        bytes
    };

    let channel_data = {
        let mut bytes = BytesMut::zeroed(1500);

        ChannelData::new(0x4000, &transaction_id).encode(&mut bytes);

        bytes
    };

    let mut turn_criterion = c.benchmark_group("turn");

    turn_criterion.throughput(Throughput::Elements(1));

    let router = Arc::new(Mutex::new(router));

    turn_criterion.bench_function("bind_request", |bencher| {
        bencher.to_async(runtime_handle).iter(|| async {
            let _ = router
                .lock()
                .route(&bind_request, local_id.source)
                .await
                .unwrap()
                .unwrap();
        })
    });

    turn_criterion.bench_function("indication", |bencher| {
        bencher.to_async(runtime_handle).iter(|| async {
            let _ = router
                .lock()
                .route(&indication, local_id.source)
                .await
                .unwrap()
                .unwrap();
        })
    });

    turn_criterion.bench_function("channel_data", |bencher| {
        bencher.to_async(runtime_handle).iter(|| async {
            let _ = router
                .lock()
                .route(&channel_data, local_id.source)
                .await
                .unwrap()
                .unwrap();
        })
    });

    turn_criterion.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
