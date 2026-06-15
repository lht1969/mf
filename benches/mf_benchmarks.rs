use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;

fn bench_encode_utf8(c: &mut Criterion) {
    let mut group = c.benchmark_group("encoding");
    let small = "Hello, World! 中文测试";
    group.bench_with_input(BenchmarkId::new("encode_utf8", "small"), &small, |b, s| {
        b.iter(|| mf::encoding::encode_string(black_box(s), "utf8"));
    });
    let large = "Hello World! ".repeat(10_000);
    group.bench_with_input(BenchmarkId::new("encode_utf8", "100KB"), &large, |b, s| {
        b.iter(|| mf::encoding::encode_string(black_box(s), "utf8"));
    });
    group.finish();
}

fn bench_detect_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_detection");
    group.bench_function("detect_json", |b| {
        let json = r#"{"key": "value", "array": [1, 2, 3], "nested": {"a": 1}}"#;
        b.iter(|| mf::content_type::detect(black_box(json)));
    });
    group.bench_function("detect_long_text", |b| {
        let text = "Hello World! ".repeat(10_000);
        b.iter(|| mf::content_type::detect(black_box(&text)));
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(3))
        .sample_size(30);
    targets = bench_encode_utf8, bench_detect_content
}
criterion_main!(benches);
