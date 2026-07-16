// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::time::Duration;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use htmlmd_core::corpus::{
    generate_docs_doc, generate_news_doc, generate_table_doc, generate_wiki_doc,
};
use htmlmd_core::{ConversionOptions, convert};

// Same allocator the CLI and server ship with, so the bench measures the
// stack users actually run.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_fixture(c: &mut Criterion, name: &str, options: &ConversionOptions) {
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
        .join(format!("{name}.html"));
    let html = fs::read_to_string(fixture).unwrap();
    let mut group = c.benchmark_group("fixture");
    group.throughput(Throughput::Bytes(html.len() as u64));
    group.bench_function(name, |b| {
        b.iter(|| convert(black_box(&html), black_box(options)).unwrap())
    });
    group.finish();
}

fn bench_corpus(c: &mut Criterion, name: &str, html: &str) {
    let profiles: &[(&str, ConversionOptions)] = &[
        ("commonmark", ConversionOptions::default()),
        ("gfm", ConversionOptions::gfm()),
        ("extended", ConversionOptions::extended()),
        ("plain-text", ConversionOptions::plain_text()),
    ];

    let mut group = c.benchmark_group(format!("corpus/{name}"));
    group.throughput(Throughput::Bytes(html.len() as u64));
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(4));

    for (profile_name, options) in profiles {
        group.bench_function(*profile_name, |b| {
            b.iter(|| convert(black_box(html), black_box(options)).unwrap())
        });
    }

    // The pre-M3 double-parse pipeline, for the historical record.
    #[cfg(feature = "backend-htmd")]
    group.bench_function("htmd-backend", |b| {
        use htmlmd_core::{HtmdBackend, convert_with_backend};
        let backend = HtmdBackend::new();
        let options = ConversionOptions::default();
        b.iter(|| convert_with_backend(black_box(html), black_box(&options), &backend).unwrap())
    });

    // Raw htmd on the same input: the "minimal library" baseline. The ratio
    // htmlmd/htmd is the wrapper overhead tracked by docs/ROADMAP.md M3.
    let raw = htmd::HtmlToMarkdown::builder().build();
    group.bench_function("raw-htmd", |b| {
        b.iter(|| raw.convert(black_box(html)).unwrap())
    });

    // Other Rust converters on identical input, for the cross-tool record
    // (docs/BENCHMARKS.md). Both are minimal single-flavor libraries.
    // NB: the fast_html2md package's library is named `html2md`.
    group.bench_function("fast_html2md", |b| {
        b.iter(|| html2md::rewrite_html(black_box(html), false))
    });
    group.bench_function("mdka", |b| {
        b.iter(|| mdka::html_to_markdown(black_box(html)))
    });

    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_fixture(c, "basic", &ConversionOptions::default());
    bench_fixture(c, "table", &ConversionOptions::gfm());
    bench_fixture(c, "malformed", &ConversionOptions::default());

    let wiki = generate_wiki_doc();
    let news = generate_news_doc();
    let docs = generate_docs_doc();
    let tables = generate_table_doc();
    eprintln!(
        "corpus sizes: wiki={} news={} docs={} tables={}",
        wiki.len(),
        news.len(),
        docs.len(),
        tables.len()
    );

    bench_corpus(c, "wiki", &wiki);
    bench_corpus(c, "news", &news);
    bench_corpus(c, "docs", &docs);
    bench_corpus(c, "tables", &tables);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
