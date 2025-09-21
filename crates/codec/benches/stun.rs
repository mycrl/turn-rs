use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use turn_server_codec::Decoder;

fn criterion_benchmark(c: &mut Criterion) {
    let mut decoder = Decoder::default();

    #[rustfmt::skip]
    let mut samples = [
        include_bytes!("../../../tests/samples/BindingRequest.bin").as_slice(),
        include_bytes!("../../../tests/samples/BindingResponse.bin").as_slice(),
        include_bytes!("../../../tests/samples/UnauthorizedAllocateRequest.bin").as_slice(),
        include_bytes!("../../../tests/samples/UnauthorizedAllocateResponse.bin").as_slice(),
        include_bytes!("../../../tests/samples/AllocateRequest.bin").as_slice(),
        include_bytes!("../../../tests/samples/AllocateResponse.bin").as_slice(),
        include_bytes!("../../../tests/samples/CreatePermissionRequest.bin").as_slice(),
        include_bytes!("../../../tests/samples/CreatePermissionResponse.bin").as_slice(),
        include_bytes!("../../../tests/samples/ChannelBindRequest.bin").as_slice(),
        include_bytes!("../../../tests/samples/ChannelBindResponse.bin").as_slice(),
        include_bytes!("../../../tests/samples/DataIndication.bin").as_slice(),
        include_bytes!("../../../tests/samples/SendIndication.bin").as_slice(),
        include_bytes!("../../../tests/samples/RefreshRequest.bin").as_slice(),
        include_bytes!("../../../tests/samples/RefreshResponse.bin").as_slice(),
    ]
    .into_iter()
    .cycle();

    let mut stun_criterion = c.benchmark_group("stun");

    stun_criterion.throughput(Throughput::Elements(1));
    stun_criterion.bench_function("decode_all_simples", |bencher| {
        bencher.iter(|| {
            decoder.decode(samples.next().unwrap()).unwrap();
        })
    });

    stun_criterion.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
