use criterion::*;
use tests::{create_turn_server, AuthMethod, TurnClient};

fn criterion_benchmark(c: &mut Criterion) {
    create_turn_server(&AuthMethod::Static);

    let mut local = TurnClient::new(&AuthMethod::Static);
    let mut peer = TurnClient::new(&AuthMethod::Static);

    let local_port = local.allocate_request();
    let peer_port = peer.allocate_request();

    local.create_permission_request(peer_port);
    peer.create_permission_request(local_port);

    let mut turn_relay = c.benchmark_group("turn_relay");
    turn_relay.bench_function("send_indication_local_to_peer", |b| {
        b.iter(|| local.indication(&peer, peer_port))
    });

    turn_relay.bench_function("send_indication_peer_to_local", |b| {
        b.iter(|| peer.indication(&local, local_port))
    });

    turn_relay.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
