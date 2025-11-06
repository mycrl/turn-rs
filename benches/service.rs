use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::net::SocketAddr;
use turn_server::{
    codec::{
        crypto::Password,
        message::attributes::PasswordAlgorithm,
    },
    service::{Service, ServiceHandler, ServiceOptions},
};

/// Mock handler for benchmarking - minimal overhead
#[derive(Clone)]
struct BenchHandler;

impl ServiceHandler for BenchHandler {
    async fn get_password(
        &self,
        _username: &str,
        algorithm: PasswordAlgorithm,
    ) -> Option<Password> {
        Some(match algorithm {
            PasswordAlgorithm::Md5 => Password::Md5([0u8; 16]),
            PasswordAlgorithm::Sha256 => Password::Sha256([0u8; 32]),
        })
    }
}

fn create_service() -> Service<BenchHandler> {
    Service::new(ServiceOptions {
        port_range: (49152..65535).into(),
        realm: "benchmark".to_string(),
        interfaces: vec!["127.0.0.1:3478".parse().unwrap()],
        handler: BenchHandler,
    })
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("service");
    group.throughput(Throughput::Elements(1));

    // Test Binding Request -> Response
    group.bench_function("binding_request_response", |bencher| {
        let service = create_service();
        let endpoint: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let interface: SocketAddr = "127.0.0.1:3478".parse().unwrap();
        let mut router = service.make_router(endpoint, interface);
        
        let binding_request = include_bytes!("../tests/samples/BindingRequest.bin");
        
        bencher.iter(|| {
            let result = pollster::block_on(async {
                router.route(binding_request, endpoint).await
            });
            std::hint::black_box(result);
        });
    });

    // Test Allocate Request -> Response (unauthorized)
    group.bench_function("allocate_unauthorized", |bencher| {
        let service = create_service();
        let endpoint: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let interface: SocketAddr = "127.0.0.1:3478".parse().unwrap();
        let mut router = service.make_router(endpoint, interface);
        
        let allocate_request = include_bytes!("../tests/samples/UnauthorizedAllocateRequest.bin");
        
        bencher.iter(|| {
            let result = pollster::block_on(async {
                router.route(allocate_request, endpoint).await
            });
            std::hint::black_box(result);
        });
    });

    // Test Allocate Request -> Response (authorized)
    group.bench_function("allocate_authorized", |bencher| {
        let service = create_service();
        let endpoint: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let interface: SocketAddr = "127.0.0.1:3478".parse().unwrap();
        let mut router = service.make_router(endpoint, interface);
        
        let allocate_request = include_bytes!("../tests/samples/AllocateRequest.bin");
        
        bencher.iter(|| {
            let result = pollster::block_on(async {
                router.route(allocate_request, endpoint).await
            });
            std::hint::black_box(result);
        });
    });

    // Test CreatePermission Request -> Response
    group.bench_function("create_permission", |bencher| {
        let service = create_service();
        let endpoint: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let interface: SocketAddr = "127.0.0.1:3478".parse().unwrap();
        let mut router = service.make_router(endpoint, interface);
        
        let create_permission_request = include_bytes!("../tests/samples/CreatePermissionRequest.bin");
        
        bencher.iter(|| {
            let result = pollster::block_on(async {
                router.route(create_permission_request, endpoint).await
            });
            std::hint::black_box(result);
        });
    });

    // Test ChannelBind Request -> Response
    group.bench_function("channel_bind", |bencher| {
        let service = create_service();
        let endpoint: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let interface: SocketAddr = "127.0.0.1:3478".parse().unwrap();
        let mut router = service.make_router(endpoint, interface);
        
        let channel_bind_request = include_bytes!("../tests/samples/ChannelBindRequest.bin");
        
        bencher.iter(|| {
            let result = pollster::block_on(async {
                router.route(channel_bind_request, endpoint).await
            });
            std::hint::black_box(result);
        });
    });

    // Test Refresh Request -> Response
    group.bench_function("refresh", |bencher| {
        let service = create_service();
        let endpoint: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let interface: SocketAddr = "127.0.0.1:3478".parse().unwrap();
        let mut router = service.make_router(endpoint, interface);
        
        let refresh_request = include_bytes!("../tests/samples/RefreshRequest.bin");
        
        bencher.iter(|| {
            let result = pollster::block_on(async {
                router.route(refresh_request, endpoint).await
            });
            std::hint::black_box(result);
        });
    });

    // Test DataIndication
    group.bench_function("data_indication", |bencher| {
        let service = create_service();
        let endpoint: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let interface: SocketAddr = "127.0.0.1:3478".parse().unwrap();
        let mut router = service.make_router(endpoint, interface);
        
        let data_indication = include_bytes!("../tests/samples/DataIndication.bin");
        
        bencher.iter(|| {
            let result = pollster::block_on(async {
                router.route(data_indication, endpoint).await
            });
            std::hint::black_box(result);
        });
    });

    // Test SendIndication
    group.bench_function("send_indication", |bencher| {
        let service = create_service();
        let endpoint: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let interface: SocketAddr = "127.0.0.1:3478".parse().unwrap();
        let mut router = service.make_router(endpoint, interface);
        
        let send_indication = include_bytes!("../tests/samples/SendIndication.bin");
        
        bencher.iter(|| {
            let result = pollster::block_on(async {
                router.route(send_indication, endpoint).await
            });
            std::hint::black_box(result);
        });
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
