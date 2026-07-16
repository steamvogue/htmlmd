// SPDX-License-Identifier: MIT OR Apache-2.0
#![cfg(feature = "backend-htmd")]

//! Differential parity tests: `NativeBackend` must produce **byte-identical**
//! Markdown to `HtmdBackend` for every fixture under every profile, for the
//! synthetic benchmark corpus, and for the option combinations that activate
//! the ported custom handlers (reference links/images, custom rules,
//! heading offset).
//!
//! Both backends convert the same *cleaned* HTML string (the output of
//! `clean_html` with the profile's own options), exactly as `convert()` feeds
//! its backend, so any difference is a rendering-port bug, never a cleanup
//! difference.

use std::fmt::Write as _;
use std::fs;

use htmlmd_core::diagnostic::Diagnostic;
use htmlmd_core::options::{
    CustomRule, CustomRuleAction, ImageMode, LinkStyle, ReferencePlacement, TitleHandling,
};
use htmlmd_core::{ConversionOptions, ConverterBackend, HtmdBackend, NativeBackend, clean_html};

fn fixture_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
}

fn fixture_paths() -> Vec<std::path::PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(fixture_dir())
        .expect("fixtures directory")
        .map(|entry| entry.expect("fixture dir entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("html"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no *.html fixtures found in fixtures/");
    paths
}

/// Clean `html` with `options`, then convert the cleaned string with both
/// backends and require byte-identical Markdown.
fn assert_parity_with_options(name: &str, html: &str, options: &ConversionOptions) {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let (cleaned, _metadata) =
        clean_html(html, options, None, &mut diagnostics).unwrap_or_else(|e| {
            panic!("clean_html failed for {name}: {e}");
        });

    let htmd_md = HtmdBackend::new()
        .convert(&cleaned, options)
        .unwrap_or_else(|e| panic!("HtmdBackend failed for {name}: {e}"))
        .markdown;
    let native_md = NativeBackend::new()
        .convert(&cleaned, options)
        .unwrap_or_else(|e| panic!("NativeBackend failed for {name}: {e}"))
        .markdown;

    assert_eq!(
        htmd_md, native_md,
        "HtmdBackend and NativeBackend outputs differ for {name}"
    );
}

fn assert_backend_parity(name: &str, html: &str) {
    assert_parity_with_options(name, html, &ConversionOptions::default());
}

/// Run every fixture through both backends with `options`.
fn assert_all_fixtures_parity(profile_name: &str, options: &ConversionOptions) {
    for path in fixture_paths() {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<non-utf8 name>")
            .to_string();
        let html = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"));
        assert_parity_with_options(&format!("{name} [{profile_name}]"), &html, options);
    }
}

#[test]
fn every_fixture_is_byte_identical_across_backends() {
    assert_all_fixtures_parity("default", &ConversionOptions::default());
}

// ---------------------------------------------------------------------------
// Profile matrix: every fixture x every profile constructor
// ---------------------------------------------------------------------------

#[test]
fn profile_commonmark_fixture_parity() {
    assert_all_fixtures_parity("commonmark", &ConversionOptions::commonmark());
}

#[test]
fn profile_gfm_fixture_parity() {
    assert_all_fixtures_parity("gfm", &ConversionOptions::gfm());
}

#[test]
fn profile_extended_fixture_parity() {
    assert_all_fixtures_parity("extended", &ConversionOptions::extended());
}

#[test]
fn profile_pandoc_fixture_parity() {
    // Faithful translation mode: exercises the native HTML serialization
    // paths (inline start/end tags and block subtree serialization).
    assert_all_fixtures_parity("pandoc", &ConversionOptions::pandoc());
}

#[test]
fn profile_obsidian_fixture_parity() {
    assert_all_fixtures_parity("obsidian", &ConversionOptions::obsidian());
}

#[test]
fn profile_mdx_safe_fixture_parity() {
    assert_all_fixtures_parity("mdx-safe", &ConversionOptions::mdx_safe());
}

#[test]
fn profile_plain_text_fixture_parity() {
    assert_all_fixtures_parity("plain-text", &ConversionOptions::plain_text());
}

// ---------------------------------------------------------------------------
// Option combinations activating specific ported handlers
// ---------------------------------------------------------------------------

fn links_fixture() -> String {
    fs::read_to_string(fixture_dir().join("links.html")).unwrap()
}

fn reference_link_options(placement: ReferencePlacement) -> ConversionOptions {
    let mut options = ConversionOptions::default();
    options.render.link_style = LinkStyle::Reference;
    options.render.reference_placement = placement;
    options.render.title_attribute = TitleHandling::Inline;
    options
}

#[test]
fn reference_links_end_placement_parity() {
    assert_parity_with_options(
        "links.html [reference/end]",
        &links_fixture(),
        &reference_link_options(ReferencePlacement::End),
    );
}

#[test]
fn reference_links_section_end_placement_parity() {
    let html = "<p><a href='https://a.com'>A</a></p><h2>Next</h2>\
                <p><a href='https://b.com' title='B site'>B</a></p>";
    assert_parity_with_options(
        "inline/section_end [reference/section-end]",
        html,
        &reference_link_options(ReferencePlacement::SectionEnd),
    );
    assert_parity_with_options(
        "links.html [reference/section-end]",
        &links_fixture(),
        &reference_link_options(ReferencePlacement::SectionEnd),
    );
}

#[test]
fn reference_links_adjacent_placement_parity() {
    assert_parity_with_options(
        "links.html [reference/adjacent]",
        &links_fixture(),
        &reference_link_options(ReferencePlacement::Adjacent),
    );
}

#[test]
fn collapsed_and_shortcut_reference_links_parity() {
    for (name, style) in [
        ("collapsed", LinkStyle::CollapsedReference),
        ("shortcut", LinkStyle::ShortcutReference),
    ] {
        let mut options = ConversionOptions::default();
        options.render.link_style = style;
        options.render.title_attribute = TitleHandling::Inline;
        assert_parity_with_options(
            &format!("links.html [reference/{name}]"),
            &links_fixture(),
            &options,
        );
    }
}

#[test]
fn reference_images_parity() {
    let mut options = ConversionOptions::default();
    options.cleanup.image_mode = ImageMode::Reference;
    options.render.title_attribute = TitleHandling::Inline;
    let html = fs::read_to_string(fixture_dir().join("image_mode.html")).unwrap();
    assert_parity_with_options("image_mode.html [image/reference]", &html, &options);

    let inline = "<p><img src='a.png' alt='A'><img src='b.png' alt='B' title='pic'></p>";
    assert_parity_with_options("inline/reference_images", inline, &options);
}

#[test]
fn custom_rules_drop_and_template_parity() {
    let mut options = ConversionOptions::extended();
    options.extension.custom_rules = vec![
        CustomRule {
            selectors: vec![".ad".to_string()],
            action: CustomRuleAction::Drop,
            template: None,
            priority: 0,
        },
        CustomRule {
            selectors: vec!["span.badge".to_string()],
            action: CustomRuleAction::MarkdownTemplate,
            template: Some("**{text}**".to_string()),
            priority: 0,
        },
        CustomRule {
            selectors: vec!["pre.tilde".to_string()],
            action: CustomRuleAction::FencedBlock,
            template: Some("txt".to_string()),
            priority: 0,
        },
    ];
    let html = "<p>Hello <span class='badge'>NEW</span> <span class='ad'>Ad</span></p>\
                <pre class='tilde'>some code</pre>";
    assert_parity_with_options("inline/custom_rules_drop_template", html, &options);
}

#[test]
fn custom_rules_selectors_and_priority_parity() {
    let mut options = ConversionOptions::extended();
    options.extension.custom_rules = vec![
        CustomRule {
            selectors: vec![".callout".to_string()],
            action: CustomRuleAction::MarkdownTemplate,
            template: Some("> {text}".to_string()),
            priority: 1,
        },
        CustomRule {
            selectors: vec!["[data-term]".to_string()],
            action: CustomRuleAction::MarkdownTemplate,
            template: Some("*{attr:data-term}*: {text}".to_string()),
            priority: 0,
        },
        CustomRule {
            selectors: vec![".callout".to_string()],
            action: CustomRuleAction::MarkdownTemplate,
            template: Some("LOSER {text}".to_string()),
            priority: 0,
        },
    ];
    let html = "<div class='callout'>heads up</div>\
                <p><span data-term='API'>application interface</span></p>";
    assert_parity_with_options("inline/custom_rules_priority", html, &options);
}

#[test]
fn custom_rules_link_and_image_parity() {
    let mut options = ConversionOptions::extended();
    options.extension.custom_rules = vec![
        CustomRule {
            selectors: vec!["a.custom-link".to_string()],
            action: CustomRuleAction::Link,
            template: None,
            priority: 0,
        },
        CustomRule {
            selectors: vec!["img.custom-img".to_string()],
            action: CustomRuleAction::Image,
            template: None,
            priority: 0,
        },
    ];
    let html = "<a class='custom-link' href='https://example.com'>Example</a>\
                <img class='custom-img' src='pic.png' alt='Pic'>";
    assert_parity_with_options("inline/custom_rules_link_image", html, &options);
}

#[test]
fn obsidian_wikilinks_parity() {
    let html = "<p>See <a class=\"wikilink\" href=\"Another page\">another note</a>, \
                <a rel=\"wikilink\" href=\"Same\">Same</a>, and \
                <a href=\"https://example.com\">a plain link</a>.</p>";
    assert_parity_with_options(
        "inline/wikilinks [obsidian]",
        html,
        &ConversionOptions::obsidian(),
    );
}

#[test]
fn mermaid_drop_policy_parity() {
    let mut options = ConversionOptions::extended();
    options.semantic.mermaid = htmlmd_core::options::MermaidPolicy::Drop;
    let html = fs::read_to_string(fixture_dir().join("mermaid.html")).unwrap();
    assert_parity_with_options("mermaid.html [mermaid/drop]", &html, &options);
}

#[test]
fn csv_table_marker_parity() {
    let mut options = ConversionOptions::extended();
    options.semantic.table_handling = htmlmd_core::options::TableHandling::CsvLike;
    let html = fs::read_to_string(fixture_dir().join("complex_table.html")).unwrap();
    assert_parity_with_options("complex_table.html [table/csv-like]", &html, &options);
}

#[test]
fn heading_offset_parity() {
    let mut options = ConversionOptions::default();
    options.semantic.heading_offset = 1;
    let html = fs::read_to_string(fixture_dir().join("basic.html")).unwrap();
    assert_parity_with_options("basic.html [heading-offset=1]", &html, &options);
}

/// Default options also activate the task-list checkbox handler
/// (`semantic.task_lists` defaults to true); no fixture contains a checkbox,
/// so pin its parity here.
#[test]
fn task_list_checkboxes_parity() {
    let html = "<ul><li><input type=\"checkbox\" checked> done</li>\
                <li><input type=\"checkbox\"> todo</li>\
                <li><input type=\"text\" value=\"not a checkbox\"> field</li></ul>";
    assert_backend_parity("inline/task_list", html);
}

// ---------------------------------------------------------------------------
// Synthetic corpus parity
//
// Generators copied from benches/convert_bench.rs: they exercise nesting,
// table, and code shapes the fixtures don't.
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

#[test]
fn corpus_wiki_doc_parity() {
    assert_backend_parity("corpus/wiki", &generate_wiki_doc());
}

#[test]
fn corpus_news_doc_parity() {
    assert_backend_parity("corpus/news", &generate_news_doc());
}

#[test]
fn corpus_docs_doc_parity() {
    assert_backend_parity("corpus/docs", &generate_docs_doc());
}

#[test]
fn corpus_tables_doc_parity() {
    assert_backend_parity("corpus/tables", &generate_table_doc());
}
