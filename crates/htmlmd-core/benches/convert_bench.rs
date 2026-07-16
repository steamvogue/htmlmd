// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt::Write as _;
use std::fs;
use std::time::Duration;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use htmlmd_core::{ConversionOptions, convert};

// Same allocator the CLI and server ship with, so the bench measures the
// stack users actually run.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// ---------------------------------------------------------------------------
// Synthetic corpus
//
// Deterministic generators approximating real-world document shapes, so the
// benchmark runs offline, in CI, and without committing megabytes of HTML.
// Sizes are chosen to match the corpus described in docs/ROADMAP.md M0.
// ---------------------------------------------------------------------------

/// ~1.4 MB Wikipedia-style article: many sections of prose with inline
/// links/emphasis, occasional tables, code samples, and footnotes.
fn generate_wiki_doc() -> String {
    let mut html = String::with_capacity(1_500_000);
    html.push_str("<html><head><title>Synthetic Wikipedia Article</title></head><body>");
    html.push_str("<h1>Synthetic Wikipedia Article</h1>");
    for section in 0..200 {
        let _ = write!(html, "<h2>Section {section}</h2>");
        for para in 0..10 {
            let _ = write!(
                html,
                "<p>Paragraph {para} of section {section} discusses the \
                 <a href=\"/wiki/Topic_{section}\">principal topic</a> in some depth, \
                 with <b>bold claims</b>, <i>italicised caveats</i>, and \
                 <a href=\"https://example.org/ref/{section}/{para}\">external references</a>. \
                 The quick brown fox jumps over the lazy dog while metrics are collected \
                 and long sentences pad the document to a realistic paragraph length \
                 comparable to encyclopedia prose, including some inline <code>code()</code> \
                 and a footnote marker<sup><a href=\"#fn{section}\">[{section}]</a></sup>.</p>"
            );
        }
        if section % 6 == 0 {
            html.push_str(
                "<table><thead><tr><th>Year</th><th>Event</th><th>Notes</th></tr></thead><tbody>",
            );
            for row in 0..10 {
                let _ = write!(
                    html,
                    "<tr><td>19{row:02}</td><td>Event {row} of section {section}</td>\
                     <td>Assorted notes about event {row}</td></tr>"
                );
            }
            html.push_str("</tbody></table>");
        }
        if section % 9 == 0 {
            let _ = write!(
                html,
                "<pre><code class=\"language-rust\">fn section_{section}() -> u32 {{\n    // demo\n    {section}\n}}\n</code></pre>"
            );
        }
    }
    html.push_str("</body></html>");
    html
}

/// ~300 KB news-style article: boilerplate chrome, tracking parameters on
/// links, lazy/srcset images, scripts and styles to strip.
fn generate_news_doc() -> String {
    let mut html = String::with_capacity(340_000);
    html.push_str(
        "<html><head><title>Synthetic News</title>\
         <meta property=\"og:title\" content=\"Synthetic News Story\">\
         <meta name=\"description\" content=\"A synthetic news article for benchmarking.\">\
         <style>body { font: 16px serif; } .ad { display: none; }</style>\
         <script>window.dataLayer = [];</script></head><body>\
         <nav><ul><li><a href=\"/\">Home</a></li><li><a href=\"/politics\">Politics</a></li></ul></nav>",
    );
    html.push_str("<article><h1>Synthetic News Story</h1>");
    for para in 0..550 {
        let _ = write!(
            html,
            "<p>Development {para}: officials said on Tuesday that \
             <a href=\"https://example.com/story/{para}?utm_source=feed&utm_medium=rss&utm_campaign=bench&fbclid=abc{para}\">the report</a> \
             would be released, according to <a href=\"https://example.org/{para}?gclid=xyz{para}\">sources</a>. \
             Analysts <em>cautioned</em> that outcomes were <strong>uncertain</strong>.</p>\
             <div class=\"ad\">ADVERTISEMENT</div>"
        );
        if para % 12 == 0 {
            let _ = write!(
                html,
                "<figure><img data-src=\"https://img.example.com/{para}.jpg\" \
                 srcset=\"https://img.example.com/{para}-480.jpg 480w, https://img.example.com/{para}-1024.jpg 1024w\" \
                 alt=\"Photo {para}\" width=\"1024\" height=\"683\">\
                 <figcaption>Caption for photo {para}</figcaption></figure>"
            );
        }
    }
    html.push_str("</article><footer><p>© Synthetic Media</p></footer></body></html>");
    html
}

/// ~500 KB API-docs page: code-block heavy with language classes.
fn generate_docs_doc() -> String {
    let mut html = String::with_capacity(540_000);
    html.push_str("<html><head><title>API Reference</title></head><body><h1>API Reference</h1>");
    for item in 0..550 {
        let _ = write!(
            html,
            "<h3><code>function_{item}()</code></h3>\
             <p>Converts input {item} into output, honouring the options described below. \
             Returns an error when the input exceeds configured limits.</p>\
             <pre><code class=\"language-rust\">pub fn function_{item}(input: &amp;str) -> Result&lt;String&gt; {{\n    let parsed = parse(input)?;\n    render(parsed, {item})\n}}\n</code></pre>\
             <pre><code>curl -X POST http://localhost:3000/convert -d '{{\"html\": \"&lt;p&gt;{item}&lt;/p&gt;\"}}'\n</code></pre>"
        );
    }
    html.push_str("</body></html>");
    html
}

/// ~400 KB table-heavy page, including complex tables (rowspan/colspan) that
/// trigger the difficult-table strategies.
fn generate_table_doc() -> String {
    let mut html = String::with_capacity(440_000);
    html.push_str("<html><head><title>Tables</title></head><body><h1>Data Tables</h1>");
    for t in 0..40 {
        let _ = write!(
            html,
            "<h2>Table {t}</h2><table><thead><tr><th>ID</th><th>Name</th><th>Value</th><th>Comment</th></tr></thead><tbody>"
        );
        for row in 0..60 {
            if t % 5 == 0 && row % 10 == 0 {
                let _ = write!(
                    html,
                    "<tr><td rowspan=\"2\">{t}-{row}</td><td colspan=\"2\">merged cell {row}</td><td>complex</td></tr>"
                );
            }
            let _ = write!(
                html,
                "<tr><td>{t}-{row}</td><td>Item {row}</td><td>{row}</td><td>Benchmark row for table {t}</td></tr>"
            );
        }
        html.push_str("</tbody></table>");
    }
    html.push_str("</body></html>");
    html
}

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
