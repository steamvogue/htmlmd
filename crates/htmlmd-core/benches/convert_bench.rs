// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use htmlmd_core::{convert, ConversionOptions};

fn bench_fixture(c: &mut Criterion, name: &str, options: &ConversionOptions) {
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
        .join(format!("{name}.html"));
    let html = fs::read_to_string(fixture).unwrap();
    let mut group = c.benchmark_group("convert");
    group.throughput(Throughput::Bytes(html.len() as u64));
    group.bench_function(name, |b| {
        b.iter(|| convert(black_box(&html), black_box(options)).unwrap())
    });
    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_fixture(c, "basic", &ConversionOptions::default());
    bench_fixture(c, "table", &ConversionOptions::gfm());
    bench_fixture(c, "malformed", &ConversionOptions::default());
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);