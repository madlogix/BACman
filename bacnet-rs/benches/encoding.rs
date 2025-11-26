use bacnet_rs::encoding::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn encode_benchmark(c: &mut Criterion) {
    c.bench_function("encode_application_tag", |b| {
        b.iter(|| {
            // Placeholder benchmark - will be implemented with actual encoding functions
            black_box(ApplicationTag::Boolean)
        })
    });
}

fn decode_benchmark(c: &mut Criterion) {
    c.bench_function("decode_application_tag", |b| {
        b.iter(|| {
            // Placeholder benchmark - will be implemented with actual decoding functions
            black_box(ApplicationTag::Boolean)
        })
    });
}

criterion_group!(benches, encode_benchmark, decode_benchmark);
criterion_main!(benches);
