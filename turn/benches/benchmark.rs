use std::{net::SocketAddr, sync::Arc};

use criterion::*;
use turn_rs::{Observer, Router};

struct RouterObserver;

impl Observer for RouterObserver {
    fn get_password_blocking(&self, _addr: &SocketAddr, _name: &str) -> Option<String> {
        Some("test".to_string())
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let router = Router::new("localhost".to_string(), Arc::new(RouterObserver));

    let local_addr: SocketAddr = "127.0.0.1:1234".parse().unwrap();
    let peer_addr: SocketAddr = "127.0.0.1:1234".parse().unwrap();
    let local_interface: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let peer_interface: SocketAddr = "127.0.0.2:0".parse().unwrap();

    let _ = router.get_key_block(&local_addr, &local_interface, &local_interface, "test");
    let _ = router.get_key_block(&peer_addr, &peer_interface, &peer_interface, "test");

    let local_port = router.alloc_port(&local_addr).unwrap();
    let peer_port = router.alloc_port(&peer_addr).unwrap();

    router.bind_port(&local_addr, peer_port);
    router.bind_port(&peer_addr, local_port);

    router.refresh(&local_addr, 600);
    router.refresh(&peer_addr, 600);

    let mut turn_router = c.benchmark_group("turn_router");
    turn_router.bench_function("local_indication_peer", |b| {
        b.iter(|| {
            let addr = router.get_port_bound(peer_port).unwrap();
            let _ = router.get_bound_port(&local_addr, &addr).unwrap();
            let _ = router.get_interface(&addr).unwrap();
        })
    });

    turn_router.bench_function("peer_indication_local", |b| {
        b.iter(|| {
            let addr = router.get_port_bound(local_port).unwrap();
            let _ = router.get_bound_port(&peer_addr, &addr).unwrap();
            let _ = router.get_interface(&addr).unwrap();
        })
    });

    router.bind_channel(&local_addr, peer_port, 0x4000).unwrap();
    router.bind_channel(&peer_addr, local_port, 0x4000).unwrap();

    turn_router.bench_function("local_channel_data_peer", |b| {
        b.iter(|| {
            let addr = router.get_channel_bound(&local_addr, 0x4000).unwrap();
            let _ = router.get_interface(&addr).unwrap();
        })
    });

    turn_router.bench_function("peer_channel_data_local", |b| {
        b.iter(|| {
            let addr = router.get_channel_bound(&peer_addr, 0x4000).unwrap();
            let _ = router.get_interface(&addr).unwrap();
        })
    });

    router.refresh(&local_addr, 0);
    router.refresh(&peer_addr, 0);
    turn_router.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
