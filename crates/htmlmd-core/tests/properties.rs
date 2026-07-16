// SPDX-License-Identifier: MIT OR Apache-2.0

//! Property-based and robustness tests (ROADMAP M4 item 2).
//!
//! Properties covered:
//! 1. `convert` with default options never panics and never returns `Err`
//!    on arbitrary input (limits only warn in non-strict mode), and running
//!    it twice yields byte-identical output (determinism).
//! 2. The same holds for every public profile constructor (profile matrix).
//! 3. Strict mode + limits are always honored: `max_input_bytes = 1` and
//!    `max_dom_depth = 1` must yield `Err`, never `Ok`, never a panic.
//! 4. Re-conversion stability: feeding the Markdown output of each fixture
//!    back through `convert` succeeds and reaches a fixpoint within one
//!    extra round.
//! 5. A deterministic garbage sweep: a fixed table of hostile inputs must
//!    convert `Ok` under every profile.
//!
//! Proptest case counts are kept deliberately small (CI on low-power
//! hardware); the suite is about robustness, not exhaustiveness.

use std::fs;
use std::path::PathBuf;

use htmlmd_core::options::Limits;
use htmlmd_core::{ConversionOptions, convert};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Every public profile constructor, labeled for diagnostics.
fn profiles() -> Vec<(&'static str, ConversionOptions)> {
    vec![
        ("commonmark", ConversionOptions::commonmark()),
        ("gfm", ConversionOptions::gfm()),
        ("extended", ConversionOptions::extended()),
        ("pandoc", ConversionOptions::pandoc()),
        ("obsidian", ConversionOptions::obsidian()),
        ("mdx_safe", ConversionOptions::mdx_safe()),
        ("plain_text", ConversionOptions::plain_text()),
    ]
}

/// Assert `convert` returns `Ok` (options are non-strict, so limits may only
/// warn) and that a second run produces byte-identical Markdown.
fn check_ok_and_deterministic(
    input: &str,
    profile: &str,
    options: &ConversionOptions,
) -> Result<(), TestCaseError> {
    let first = match convert(input, options) {
        Ok(result) => result,
        Err(e) => {
            return Err(TestCaseError::fail(format!(
                "profile {profile}: convert returned Err on non-strict options: {e}"
            )));
        }
    };
    let second = match convert(input, options) {
        Ok(result) => result,
        Err(e) => {
            return Err(TestCaseError::fail(format!(
                "profile {profile}: second convert returned Err: {e}"
            )));
        }
    };
    prop_assert_eq!(
        &first.markdown,
        &second.markdown,
        "profile {} produced different output across two runs",
        profile
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Input strategies
// ---------------------------------------------------------------------------

/// Truly arbitrary input: random bytes, lossily decoded to UTF-8.
fn arbitrary_text() -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<u8>(), 0..2048)
        .prop_map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// Building blocks for grammar-ish pseudo-HTML: open/close tags (often
/// mismatched or unclosed), half-written attributes, entities (valid and
/// broken), Markdown metacharacters, nested brackets, emoji, and
/// direction-control characters.
const HTML_FRAGMENTS: &[&str] = &[
    "<div>",
    "</div>",
    "<p>",
    "</p>",
    "<span>",
    "<a href='",
    "<a href=\"https://example.com/a?b=c&d=e\">",
    "'>",
    "\">",
    "</a>",
    "<ul><li>",
    "</li></ul>",
    "<ol start='3'><li>",
    "<h1>",
    "</h2>",
    "<blockquote>",
    "<pre><code class=\"language-rust\">",
    "</code></pre>",
    "<table><tr><td>",
    "</td></tr></table>",
    "<img src='x.png' alt='[alt]'>",
    "<br>",
    "<hr/>",
    "<!--",
    "-->",
    "<![CDATA[",
    "]]>",
    "<script>",
    "</script>",
    "<style>",
    "</style>",
    "<details><summary>",
    "<input type=checkbox checked>",
    "&amp;",
    "&nbsp;",
    "&#x27;",
    "&#xZZ;",
    "&#999999999;",
    "&bogus;",
    "&",
    "plain words here",
    "*stars* _unders_ `ticks` ~~strike~~",
    "[[nested [brackets]] and](paren",
    "![img](",
    "| pipe | cells |",
    "# not a heading?",
    "emoji \u{1F600}\u{1F389}\u{1F1EC}\u{1F1E7}",
    "\u{202E}rtl\u{202C}",
    "\u{200B}\u{200C}\u{200D}\u{FEFF}",
    "\u{FFFD}",
    "line\nbreak",
    "\ttab and  spaces ",
    "\\back\\slash",
    "<",
    ">",
    "</",
    "<<<",
    ">>>",
    "='",
];

/// Pseudo-HTML: a random concatenation of fragments plus raw text noise.
fn pseudo_html() -> impl Strategy<Value = String> {
    let fragment = prop_oneof![
        4 => proptest::sample::select(HTML_FRAGMENTS).prop_map(str::to_owned),
        1 => "[ -~]{0,32}",  // printable-ASCII noise
        1 => "\\PC{0,8}",    // arbitrary non-control Unicode
    ];
    proptest::collection::vec(fragment, 0..40).prop_map(|parts| parts.concat())
}

// ---------------------------------------------------------------------------
// Property 1: no-panic + determinism on arbitrary input (default options)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    #[test]
    fn arbitrary_bytes_convert_ok_and_deterministic(input in arbitrary_text()) {
        check_ok_and_deterministic(&input, "commonmark", &ConversionOptions::default())?;
    }

    #[test]
    fn pseudo_html_convert_ok_and_deterministic(input in pseudo_html()) {
        check_ok_and_deterministic(&input, "commonmark", &ConversionOptions::default())?;
    }
}

// ---------------------------------------------------------------------------
// Property 2: profile matrix no-panic + determinism
// ---------------------------------------------------------------------------

proptest! {
    // Each case fans out to 7 profiles x 2 runs, so keep the count lower.
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    #[test]
    fn profile_matrix_convert_ok_and_deterministic(input in pseudo_html()) {
        for (profile, options) in profiles() {
            check_ok_and_deterministic(&input, profile, &options)?;
        }
    }
}

// ---------------------------------------------------------------------------
// Property 3: strict limits are always honored
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    #[test]
    fn strict_max_input_bytes_always_errors(body in pseudo_html()) {
        // The wrapper guarantees the input is longer than 1 byte.
        let input = format!("<p>{body}</p>");
        let options = ConversionOptions {
            strict: true,
            limits: Limits {
                max_input_bytes: 1,
                ..Limits::default()
            },
            ..ConversionOptions::default()
        };
        match convert(&input, &options) {
            Err(_) => {}
            Ok(_) => {
                return Err(TestCaseError::fail(
                    "strict max_input_bytes=1 must always return Err",
                ));
            }
        }
    }

    #[test]
    fn strict_max_dom_depth_always_errors(body in "[a-z ]{0,32}") {
        // Nested input guarantees a DOM depth well above 1 (the parser alone
        // produces html > body > div > div > div).
        let input = format!("<div><div><div>{body}</div></div></div>");
        let options = ConversionOptions {
            strict: true,
            limits: Limits {
                max_dom_depth: 1,
                ..Limits::default()
            },
            ..ConversionOptions::default()
        };
        match convert(&input, &options) {
            Err(_) => {}
            Ok(_) => {
                return Err(TestCaseError::fail(
                    "strict max_dom_depth=1 must always return Err on nested input",
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property 4: re-conversion stability over the fixture corpus
// ---------------------------------------------------------------------------

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
}

/// Fixtures whose first-pass Markdown contains Markdown metacharacters
/// (`#`, `*`, `[`, `` ` ``, `|`, ...). Feeding that Markdown back in as HTML
/// correctly backslash-escapes those characters in the new output, and the
/// round after that must escape the backslashes themselves
/// (`` \` `` -> `` \\\` ``), so the output grows every round and no fixpoint
/// exists. That is correct text-escaping semantics (each output faithfully
/// represents the previous output as literal text), not a determinism bug,
/// so for these fixtures only Ok/no-panic is asserted.
const ESCAPE_COMPOUNDING_FIXTURES: &[&str] = &[
    "basic",
    "code",
    "code_detection",
    "complex_table",
    "extended",
    "image_mode",
    "lazy_image",
    "links",
    "malformed",
    "mermaid",
    "metadata",
    "nested_lists",
    "srcset",
];

#[test]
fn fixture_reconversion_reaches_fixpoint() {
    let options = ConversionOptions::default();
    let mut seen: Vec<String> = Vec::new();
    for entry in fs::read_dir(fixture_dir()).expect("fixtures directory must exist") {
        let path = entry.expect("readable dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("html") {
            continue;
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let html = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("fixture {name}: read failed: {e}"));

        let md1 = convert(&html, &options)
            .unwrap_or_else(|e| panic!("fixture {name}: initial conversion failed: {e}"))
            .markdown;
        // Feeding Markdown back through the HTML converter must not panic
        // and must succeed.
        let md2 = convert(&md1, &options)
            .unwrap_or_else(|e| panic!("fixture {name}: re-conversion failed: {e}"))
            .markdown;
        // One more round must be a fixpoint (except for the documented
        // escape-compounding fixtures, where only Ok/no-panic is asserted).
        let md3 = convert(&md2, &options)
            .unwrap_or_else(|e| panic!("fixture {name}: third conversion failed: {e}"))
            .markdown;
        if !ESCAPE_COMPOUNDING_FIXTURES.contains(&name.as_str()) {
            assert_eq!(
                md3, md2,
                "fixture {name}: re-conversion did not reach a fixpoint within one extra round"
            );
        }
        seen.push(name);
    }
    assert!(
        !seen.is_empty(),
        "no *.html fixtures found in {:?}",
        fixture_dir()
    );
    // Keep the exception list honest: every listed fixture must still exist.
    for name in ESCAPE_COMPOUNDING_FIXTURES {
        assert!(
            seen.iter().any(|s| s == name),
            "stale entry {name:?} in ESCAPE_COMPOUNDING_FIXTURES: fixture no longer exists"
        );
    }
}

// ---------------------------------------------------------------------------
// Property 5: deterministic garbage sweep (fixed nasty inputs, all profiles)
// ---------------------------------------------------------------------------

fn nasty_inputs() -> Vec<(&'static str, String)> {
    vec![
        ("empty", String::new()),
        ("lone_lt", "<".to_string()),
        ("angle_soup", "<<<>>>".to_string()),
        ("null_bytes", "\0\0\0".to_string()),
        ("null_in_markup", "a\0b<p>c\0</p>".to_string()),
        ("replacement_chars", "\u{FFFD}\u{FFFD}\u{FFFD}".to_string()),
        ("unterminated_comment", "<!--".to_string()),
        ("comment_swallows_doc", "<!-- <p>never closed".to_string()),
        ("unclosed_script", "<script>".to_string()),
        (
            "script_writes_html",
            "<script>document.write('<p>x</p>')</script>".to_string(),
        ),
        ("style_block", "<style>p { color: red }</style>".to_string()),
        ("unclosed_table", "<table><tr>".to_string()),
        ("orphan_cell", "<table><td>orphan cell".to_string()),
        (
            "pre_control_chars",
            format!(
                "<pre>{}</pre>",
                "\u{1}\u{2}\u{7}\u{8}\u{B}\u{C}\u{E}\u{1B}\u{7F}"
            ),
        ),
        (
            "rtl_and_zero_width",
            "<p>\u{202E}evil\u{202C} \u{200B}\u{200C}\u{200D}\u{FEFF}</p>".to_string(),
        ),
        ("cdata_close", "]]>".to_string()),
        ("cdata_open", "<![CDATA[ <p>hi</p> ]]>".to_string()),
        ("broken_hex_entity", "&#xZZ;".to_string()),
        ("surrogate_entity", "&#xD800;".to_string()),
        ("out_of_range_entity", "&#x110000;".to_string()),
        ("unterminated_entities", "&amp &lt &nosemi".to_string()),
        ("unknown_entity", "&bogusentity;".to_string()),
        ("ampersand_run", format!("<p>{}</p>", "&".repeat(64))),
        (
            "javascript_href",
            "<a href='javascript:alert(1)'>x</a>".to_string(),
        ),
        ("unterminated_attr", "<a href='".to_string()),
        (
            "unterminated_quoted_class",
            "<div class=\"unterminated".to_string(),
        ),
        (
            "angle_in_attr",
            "<p title='a<b>c'>attr with angle brackets</p>".to_string(),
        ),
        (
            "markdown_metachars",
            "<p>[link](url) *md* _in_ `html` | pipes | #hash</p>".to_string(),
        ),
        ("close_only_tags", "</div></p></body></html>".to_string()),
        (
            "doubled_documents",
            "<html><body><html><body>doubled documents".to_string(),
        ),
        (
            "svg_foreign_object",
            "<svg><foreignObject><p>fo</p></foreignObject></svg>".to_string(),
        ),
        (
            "mathml",
            "<math><mi>x</mi><mo>+</mo><mn>1</mn></math>".to_string(),
        ),
        ("bom_prefixed", "\u{FEFF}<p>BOM prefixed</p>".to_string()),
        (
            "emoji_and_combining",
            "<p>e\u{301}\u{301} \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467} \u{1F1FA}\u{1F1F3}</p>"
                .to_string(),
        ),
        (
            "double_doctype",
            "<!DOCTYPE html><!DOCTYPE html><p>double doctype</p>".to_string(),
        ),
        (
            "processing_instruction",
            "<?php echo 'x'; ?><p>after pi</p>".to_string(),
        ),
    ]
}

#[test]
fn garbage_sweep_all_profiles() {
    for (label, input) in nasty_inputs() {
        for (profile, options) in profiles() {
            let result = convert(&input, &options);
            assert!(
                result.is_ok(),
                "input {label:?} under profile {profile}: expected Ok, got {:?}",
                result.err()
            );
        }
    }
}

/// Deep nesting is a separate test: with default (unlimited) limits it must
/// still convert Ok — depth produces at most warnings, never errors, in
/// non-strict mode.
///
/// KNOWN BUG (why this test is ignored): the native renderer walks the DOM
/// with unbounded mutual recursion (`dom_walker::walk_node` ->
/// `ElementHandlers::handle` -> `block_handler` -> `walk_children` ->
/// `walk_node`, ~8 stack frames per DOM level), so deeply nested input
/// overflows the stack and aborts the whole process — it cannot even return
/// an `Err`. Minimal repro:
///
/// ```ignore
/// let input = "<div>".repeat(1900); // closing tags optional
/// let _ = htmlmd_core::convert(&input, &ConversionOptions::default());
/// // fatal runtime error: stack overflow (SIGABRT)
/// ```
///
/// Deep nesting must not abort the process.
///
/// Regression test for a real stack overflow: the renderer recurses per DOM
/// level, so before `max_dom_depth` got a non-zero default (and an enforcing
/// prune pass in cleanup) `"<div>".repeat(1900)` aborted with SIGABRT rather
/// than returning. Default limits now cut over-deep subtrees with a warning.
///
/// Depths stay modest on purpose: html5ever's tree builder is quadratic in
/// nesting depth (50k nested divs parse in ~10 s even in release), so a
/// deeper case would buy no extra coverage of *our* code at a large cost in
/// CI time. 2 000 clears the ~1 900 abort threshold with headroom.
#[test]
fn deeply_nested_divs_default_limits_ok() {
    let input = format!("{}deep{}", "<div>".repeat(2_000), "</div>".repeat(2_000));
    for (profile, options) in profiles() {
        let result = convert(&input, &options);
        assert!(
            result.is_ok(),
            "2000-deep nested divs under profile {profile}: expected Ok, got {:?}",
            result.err()
        );
    }
}

/// The depth limit is enforced, not merely reported: over-deep content is
/// pruned (with a diagnostic) in non-strict mode.
#[test]
fn depth_limit_prunes_and_warns() {
    let input = format!("{}deep{}", "<div>".repeat(400), "</div>".repeat(400));
    let result = convert(&input, &ConversionOptions::default()).expect("convert");
    assert!(
        !result.markdown.contains("deep"),
        "content nested past max-dom-depth should be pruned, got {:?}",
        result.markdown
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("max-dom-depth")),
        "pruning must emit a diagnostic, got {:?}",
        result.diagnostics
    );

    // Within the limit, nothing is touched.
    let shallow = format!("{}deep{}", "<div>".repeat(10), "</div>".repeat(10));
    let result = convert(&shallow, &ConversionOptions::default()).expect("convert");
    assert!(result.markdown.contains("deep"));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

/// Large flat input: one megabyte of 'a' must convert Ok under every profile.
#[test]
fn megabyte_of_text_ok() {
    let input = "a".repeat(1 << 20);
    for (profile, options) in profiles() {
        let result = convert(&input, &options);
        assert!(
            result.is_ok(),
            "1MB text input under profile {profile}: expected Ok, got {:?}",
            result.err()
        );
    }
}
