use criterion::*;
use tests::{create_turn_server, AuthMethod, TurnClient};

fn criterion_benchmark(c: &mut Criterion) {
    let bind = "127.0.0.1:3578".parse().unwrap();
    create_turn_server(&AuthMethod::Static, bind);
    let username = "user1";
    let mut local = TurnClient::new(&AuthMethod::Static, bind, username);
    let mut peer = TurnClient::new(&AuthMethod::Static, bind, username);

    let local_port = local.allocate_request();
    let peer_port = peer.allocate_request();

    local.create_permission_request(username);
    peer.create_permission_request(username);

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
