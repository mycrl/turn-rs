use criterion::*;
use tests::{allocate_request, create_client, create_permission_request, create_turn, indication};
use tokio::{net::UdpSocket, runtime::Runtime};

fn create_turn_block(rt: &Runtime) {
    rt.block_on(async { create_turn().await })
}

fn create_client_block(rt: &Runtime) -> UdpSocket {
    rt.block_on(async { create_client().await })
}

fn allocate_request_block(rt: &Runtime, socket: &UdpSocket) -> u16 {
    rt.block_on(async { allocate_request(&socket).await })
}

fn create_permission_request_block(rt: &Runtime, socket: &UdpSocket, port: u16) {
    rt.block_on(async { create_permission_request(&socket, port).await })
}

fn criterion_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    create_turn_block(&rt);
    let local = create_client_block(&rt);
    let peer = create_client_block(&rt);
    let local_port = allocate_request_block(&rt, &local);
    let peer_port = allocate_request_block(&rt, &peer);
    create_permission_request_block(&rt, &local, peer_port);
    create_permission_request_block(&rt, &peer, local_port);

    let mut turn_relay = c.benchmark_group("turn_relay");
    turn_relay.bench_function("send_indication_local_to_peer", |b| {
        b.to_async(&rt)
            .iter(|| indication(&local, &peer, peer_port))
    });

    turn_relay.bench_function("send_indication_peer_to_local", |b| {
        b.to_async(&rt)
            .iter(|| indication(&peer, &local, local_port))
    });

    turn_relay.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
