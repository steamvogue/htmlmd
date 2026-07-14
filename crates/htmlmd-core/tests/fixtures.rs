// SPDX-License-Identifier: MIT OR Apache-2.0

use htmlmd_core::{
    convert, ConversionOptions,
    options::{DetailsHandling, FormHandling, ImageMode, MediaPolicy},
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
