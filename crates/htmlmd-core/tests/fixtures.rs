// SPDX-License-Identifier: MIT OR Apache-2.0

use htmlmd_core::{
    convert, ConversionOptions,
    options::{
        CustomRule, CustomRuleAction, DetailsHandling, DifficultTableStrategy, FormHandling,
        ImageMode, MediaPolicy, MermaidPolicy, TableHandling,
    },
};
use std::fs;

fn fixture_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
}

fn fixture(name: &str, options: &ConversionOptions) -> String {
    let html = fs::read_to_string(fixture_dir().join(format!("{name}.html"))).unwrap();
    convert(&html, options).unwrap().markdown
}

fn expected(name: &str) -> String {
    fs::read_to_string(fixture_dir().join("expected").join(format!("{name}.md"))).unwrap()
}

#[test]
fn basic_commonmark() {
    assert_eq!(fixture("basic", &ConversionOptions::default()), expected("basic"));
}

#[test]
fn malformed_html() {
    assert_eq!(fixture("malformed", &ConversionOptions::default()), expected("malformed"));
}

#[test]
fn gfm_table() {
    assert_eq!(fixture("table", &ConversionOptions::gfm()), expected("table"));
}

#[test]
fn nested_lists() {
    assert_eq!(fixture("nested_lists", &ConversionOptions::default()), expected("nested_lists"));
}

#[test]
fn lazy_images() {
    assert_eq!(fixture("lazy_image", &ConversionOptions::default()), expected("lazy_image"));
}

#[test]
fn code_block() {
    assert_eq!(fixture("code", &ConversionOptions::default()), expected("code"));
}

#[test]
fn links_cleanup() {
    assert_eq!(fixture("links", &ConversionOptions::default()), expected("links"));
}

#[test]
fn hidden_content_removed() {
    assert_eq!(fixture("hidden", &ConversionOptions::default()), expected("hidden"));
}

#[test]
fn metadata_extraction() {
    let mut opts = ConversionOptions::default();
    opts.cleanup.metadata.title = true;
    opts.cleanup.metadata.description = true;
    opts.cleanup.metadata.canonical_url = true;
    let result = convert(
        &fs::read_to_string(fixture_dir().join("metadata.html")).unwrap(),
        &opts,
    )
    .unwrap();
    assert_eq!(result.markdown, expected("metadata"));
    assert_eq!(result.title.as_deref(), Some("Page Title"));
    assert_eq!(result.description.as_deref(), Some("Page description"));
    assert_eq!(result.canonical_url.as_deref(), Some("https://example.com/page"));
}

#[test]
fn determinism() {
    let html = fs::read_to_string(fixture_dir().join("basic.html")).unwrap();
    let a = convert(&html, &ConversionOptions::default()).unwrap().markdown;
    let b = convert(&html, &ConversionOptions::default()).unwrap().markdown;
    assert_eq!(a, b);
}

#[test]
fn image_mode_skip() {
    let mut opts = ConversionOptions::default();
    opts.cleanup.image_mode = ImageMode::Skip;
    let md = fixture("image_mode", &opts);
    assert!(!md.contains("![Photo]"));
    assert!(!md.contains("![Diagram]"));
}

#[test]
fn image_mode_alt_text() {
    let mut opts = ConversionOptions::default();
    opts.cleanup.image_mode = ImageMode::AltText;
    let md = fixture("image_mode", &opts);
    assert!(md.contains("Photo"));
    assert!(md.contains("Diagram"));
    assert!(!md.contains("!["));
}

#[test]
fn image_metadata_preserved() {
    let mut opts = ConversionOptions::default();
    opts.cleanup.preserve_image_metadata = true;
    let md = fixture("image_mode", &opts);
    assert!(md.contains("Diagram (100x200)"));
}

#[test]
fn srcset_first_candidate() {
    let md = fixture("srcset", &ConversionOptions::default());
    assert!(md.contains("small.jpg"));
    assert!(!md.contains("large.jpg"));
}

#[test]
fn srcset_largest_candidate() {
    let mut opts = ConversionOptions::default();
    opts.cleanup.responsive_image_policy = htmlmd_core::options::ResponsiveImagePolicy::Largest;
    let md = fixture("srcset", &opts);
    assert!(md.contains("large.jpg"));
    assert!(!md.contains("small.jpg"));
}

#[test]
fn details_summary_only() {
    let mut opts = ConversionOptions::default();
    opts.cleanup.details_handling = DetailsHandling::SummaryOnly;
    let md = fixture("details", &opts);
    assert!(md.contains("Click to expand"));
    assert!(!md.contains("Hidden details content"));
}

#[test]
fn details_drop() {
    let mut opts = ConversionOptions::default();
    opts.cleanup.details_handling = DetailsHandling::Drop;
    let md = fixture("details", &opts);
    assert!(!md.contains("Click to expand"));
    assert!(!md.contains("Hidden details content"));
}

#[test]
fn form_readable() {
    let mut opts = ConversionOptions::default();
    opts.cleanup.form_handling = FormHandling::Readable;
    let md = fixture("form", &opts);
    eprintln!("FORM MD: {:?}", md);
    assert!(md.contains("Name: Ada"));
    assert!(md.contains("Email:"));
}

#[test]
fn custom_elements_unwrapped() {
    let md = fixture("custom_elements", &ConversionOptions::default());
    assert!(md.contains("Before custom content after"));
    assert!(!md.contains("my-element"));
}

#[test]
fn media_drop() {
    let mut opts = ConversionOptions::default();
    opts.cleanup.media_policy = MediaPolicy::Drop;
    let html = "<p>text</p><video src='x.mp4'></video><audio src='y.mp3'></audio>";
    let md = convert(html, &opts).unwrap().markdown;
    assert!(md.contains("text"));
    assert!(!md.contains("video"));
    assert!(!md.contains("audio"));
}

#[test]
fn media_placeholder() {
    let mut opts = ConversionOptions::default();
    opts.cleanup.media_policy = MediaPolicy::Placeholder;
    let html = "<video src='x.mp4'></video>";
    let md = convert(html, &opts).unwrap().markdown;
    eprintln!("MEDIA MD: {:?}", md);
    assert!(md.contains("(VIDEO: x.mp4)"));
}


#[test]
fn extended_profile_semantic_features() {
    let md = fixture("extended", &ConversionOptions::extended());
    eprintln!("EXTENDED MD:\n{md}");
    assert!(md.contains("==important=="));
    assert!(md.contains("~~removed~~"));
    assert!(md.contains("++added++"));
    assert!(md.contains("H~2~O"));
    assert!(md.contains("E=mc^2^"));
    assert!(md.contains("<kbd>Ctrl</kbd>"));
    assert!(md.contains("[^1]"));
    assert!(md.contains("[^1]: Footnote text."));
    assert!(md.contains("Term\n: Definition text."));
    assert!(md.contains("$E=mc^2$"));
    assert!(md.contains("> [!NOTE]"));
    assert!(md.contains("> This is an alert."));
}

#[test]
fn code_language_detection() {
    let md = fixture("code_detection", &ConversionOptions::extended());
    eprintln!("CODE DETECTION MD:\n{md}");
    assert!(md.contains("```go"));
    assert!(md.contains("```python"));
    assert!(md.contains("```rust"));
    assert!(md.contains("```shell"));
}

#[test]
fn mermaid_to_fenced() {
    let mut opts = ConversionOptions::extended();
    opts.semantic.mermaid = MermaidPolicy::Fenced;
    let md = fixture("mermaid", &opts);
    eprintln!("MERMAID MD:\n{md}");
    assert!(md.contains("```mermaid"));
    assert!(md.contains("graph TD;"));
    assert!(md.contains("sequenceDiagram;"));
}

#[test]
fn complex_table_html_fallback() {
    let mut opts = ConversionOptions::extended();
    opts.semantic.difficult_table_strategy = DifficultTableStrategy::HtmlFallback;
    let md = fixture("complex_table", &opts);
    eprintln!("TABLE HTML FALLBACK MD:\n{md}");
    assert!(md.contains("```html"));
    assert!(md.contains("<table>"));
}

#[test]
fn complex_table_flatten() {
    let mut opts = ConversionOptions::extended();
    opts.semantic.table_handling = TableHandling::Flatten;
    let md = fixture("complex_table", &opts);
    eprintln!("TABLE FLATTEN MD:\n{md}");
    assert!(!md.contains("| A | B |"));
    assert!(md.contains("A | B"));
    assert!(md.contains("1 | 2"));
}

#[test]
fn complex_table_csv_like() {
    let mut opts = ConversionOptions::extended();
    opts.semantic.table_handling = TableHandling::CsvLike;
    let md = fixture("complex_table", &opts);
    eprintln!("TABLE CSV MD:\n{md}");
    assert!(md.contains("```csv"));
    assert!(md.contains("A,B"));
}

#[test]
fn custom_rules_drop_and_template() {
    let mut opts = ConversionOptions::extended();
    opts.extension.custom_rules = vec![
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
    let html = "<p>Hello <span class='badge'>NEW</span> <span class='ad'>Ad</span></p><pre class='tilde'>some code</pre>";
    let md = convert(html, &opts).unwrap().markdown;
    eprintln!("CUSTOM RULES MD:\n{md}");
    assert!(!md.contains("Ad"));
    assert!(md.contains("**NEW**"));
    assert!(md.contains("```txt"));
    assert!(md.contains("some code"));
}

#[test]
fn custom_rules_link_and_image() {
    let mut opts = ConversionOptions::extended();
    opts.extension.custom_rules = vec![
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
    let html = "<a class='custom-link' href='https://example.com'>Example</a><img class='custom-img' src='pic.png' alt='Pic'>";
    let md = convert(html, &opts).unwrap().markdown;
    eprintln!("CUSTOM LINK IMAGE MD:\n{md}");
    assert!(md.contains("[Example](https://example.com/)"));
    assert!(md.contains("![Pic](pic.png)"));
}

#[test]
fn obsidian_profile_wikilinks_and_frontmatter() {
    let mut opts = ConversionOptions::obsidian();
    opts.cleanup.metadata.title = true;
    opts.cleanup.metadata.description = true;
    opts.cleanup.metadata.canonical_url = true;
    let html = r#"<!DOCTYPE html>
<html><head><title>My Note</title>
<meta name="description" content="A note about things">
<link rel="canonical" href="https://example.com/note">
</head><body>
<p>See <a class="wikilink" href="Another page">another note</a>.</p>
</body></html>"#;
    let md = convert(html, &opts).unwrap().markdown;
    eprintln!("OBSIDIAN MD:\n{md}");
    assert!(md.contains("---"));
    assert!(md.contains("title: My Note"));
    assert!(md.contains("description: A note about things"));
    assert!(md.contains("canonical_url: https://example.com/note"));
    assert!(md.contains("[[Another page|another note]]"));
}

#[test]
fn pandoc_profile_preserves_raw_html() {
    let md = convert("<p>Press <kbd>Ctrl</kbd>.</p>", &ConversionOptions::pandoc())
        .unwrap()
        .markdown;
    eprintln!("PANDOC MD:\n{md}");
    assert!(md.contains("<kbd>Ctrl</kbd>"));
}

#[test]
fn mdx_safe_profile_escapes_jsx() {
    let md = convert(
        "<p>Press <kbd>Ctrl</kbd> and use {config.value}.</p>",
        &ConversionOptions::mdx_safe(),
    )
    .unwrap()
    .markdown;
    eprintln!("MDX SAFE MD:\n{md}");
    assert!(!md.contains("<kbd>"));
    assert!(md.contains("Ctrl"));
    assert!(md.contains(r"\{config.value\}"));
}

#[test]
fn plain_text_profile_strips_markdown() {
    let html = "<h1>Title</h1><p>A <em>simple</em> <a href='https://example.com'>link</a> and <img src='x.png' alt='diagram'>.</p><ul><li>one</li><li>two</li></ul>";
    let md = convert(html, &ConversionOptions::plain_text())
        .unwrap()
        .markdown;
    eprintln!("PLAIN TEXT MD:\n{md}");
    assert!(!md.contains('#'));
    assert!(!md.contains('*'));
    assert!(!md.contains('['));
    assert!(!md.contains('!'));
    assert!(md.contains("Title"));
    assert!(md.contains("simple"));
    assert!(md.contains("link"));
    assert!(md.contains("diagram"));
    assert!(md.contains("one"));
    assert!(md.contains("two"));
}
