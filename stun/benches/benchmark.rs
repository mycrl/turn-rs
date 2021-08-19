use stun::Decoder;
use criterion::*;

const CHANNEL_BIND: [u8; 108] = [
    0x00, 0x09, 0x00, 0x58, 0x21, 0x12, 0xa4, 0x42,
    0x35, 0x6a, 0x52, 0x42, 0x33, 0x4c, 0x65, 0x68,
    0x2b, 0x7a, 0x75, 0x52, 0x00, 0x0c, 0x00, 0x04,
    0x40, 0x00, 0x00, 0x00, 0x00, 0x12, 0x00, 0x08,
    0x00, 0x01, 0xe1, 0x10, 0x5e, 0x12, 0xa4, 0x43,
    0x00, 0x06, 0x00, 0x03, 0x64, 0x65, 0x76, 0x00,
    0x00, 0x14, 0x00, 0x09, 0x6c, 0x6f, 0x63, 0x61,
    0x6c, 0x68, 0x6f, 0x73, 0x74, 0x00, 0x00, 0x00,
    0x00, 0x15, 0x00, 0x10, 0x6c, 0x37, 0x7a, 0x38,
    0x33, 0x6b, 0x6c, 0x36, 0x61, 0x35, 0x63, 0x73,
    0x77, 0x74, 0x74, 0x34, 0x00, 0x08, 0x00, 0x14,
    0xbd, 0xb8, 0xee, 0x7d, 0xc8, 0x9f, 0x85, 0x1b,
    0x5f, 0x18, 0x9a, 0x7b, 0x84, 0x3a, 0xfd, 0x88,
    0xde, 0x03, 0xc0, 0x34
];

const BINDING: [u8; 96] = [
    0x00, 0x01, 0x00, 0x4c, 0x21, 0x12, 0xa4, 0x42, 
    0x71, 0x66, 0x46, 0x31, 0x2b, 0x59, 0x79, 0x65, 
    0x56, 0x69, 0x32, 0x72, 0x00, 0x06, 0x00, 0x09, 
    0x55, 0x43, 0x74, 0x39, 0x3a, 0x56, 0x2f, 0x2b, 
    0x2f, 0x00, 0x00, 0x00, 0xc0, 0x57, 0x00, 0x04, 
    0x00, 0x00, 0x03, 0xe7, 0x80, 0x29, 0x00, 0x08, 
    0x22, 0x49, 0xda, 0x28, 0x2c, 0x6f, 0x2e, 0xdb, 
    0x00, 0x24, 0x00, 0x04, 0x6e, 0x00, 0x28, 0xff, 
    0x00, 0x08, 0x00, 0x14, 0x19, 0x58, 0xda, 0x38, 
    0xed, 0x1e, 0xdd, 0xc8, 0x6b, 0x8e, 0x22, 0x63, 
    0x3a, 0x22, 0x63, 0x97, 0xcf, 0xf5, 0xde, 0x82, 
    0x80, 0x28, 0x00, 0x04, 0x56, 0xf7, 0xa3, 0xed
];

fn criterion_benchmark(c: &mut Criterion) {
    let mut stun_decoder = c.benchmark_group("stun_decoder");
    let mut codec = Decoder::new();
    
    let channel_bind = &CHANNEL_BIND[..];
    stun_decoder.throughput(Throughput::Bytes(channel_bind.len() as u64));
    stun_decoder.bench_function("decoder_channel_bind", |b| b.iter(|| {
        codec.decode(channel_bind).unwrap();
    }));
    
    let binding = &BINDING[..];
    stun_decoder.throughput(Throughput::Bytes(binding.len() as u64));
    stun_decoder.bench_function("decoder_binding", |b| b.iter(|| {
        codec.decode(binding).unwrap();
    }));
    
    stun_decoder.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
