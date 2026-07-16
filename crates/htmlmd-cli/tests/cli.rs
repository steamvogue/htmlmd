// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;

fn fixture_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
}

#[test]
fn converts_file_to_stdout() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg(fixture_dir().join("basic.html"));
    cmd.assert().success().stdout(contains("# Hello World"));
}

#[test]
fn converts_stdin() {
    let html = fs::read_to_string(fixture_dir().join("basic.html")).unwrap();
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("-").write_stdin(html);
    cmd.assert().success().stdout(contains("# Hello World"));
}

#[test]
fn heading_style_setex() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--heading-style", "setex"])
        .arg(fixture_dir().join("basic.html"));
    cmd.assert()
        .success()
        .stdout(contains("Hello World\n====="));
}

#[test]
fn bullet_asterisk() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--bullet", "asterisk"])
        .arg(fixture_dir().join("basic.html"));
    cmd.assert().success().stdout(contains("*   First item"));
}

#[test]
fn skip_tags_flag() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--skip-tags", "a"])
        .arg(fixture_dir().join("basic.html"));
    cmd.assert()
        .success()
        .stdout(contains("Example link").not());
}

#[test]
fn remove_selectors() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--remove-selectors", "ul"])
        .arg(fixture_dir().join("basic.html"));
    cmd.assert().success().stdout(contains("First item").not());
}

#[test]
fn extract_selector() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--extract-selector", "ul"])
        .arg(fixture_dir().join("basic.html"));
    cmd.assert()
        .success()
        .stdout(contains("First item"))
        .stdout(contains("Hello World").not());
}

#[test]
fn base_url_resolves_relative() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--base-url", "https://example.com/blog/"])
        .arg(fixture_dir().join("links.html"));
    cmd.assert()
        .success()
        .stdout(contains("https://example.com/page"));
}

#[test]
fn tracking_params_stripped() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg(fixture_dir().join("links.html"));
    cmd.assert()
        .success()
        .stdout(contains("utm_source").not())
        .stdout(contains("https://example.com/page"));
}

#[test]
fn print_default_config() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("--print-default-config");
    cmd.assert().success().stdout(contains("profile"));
}

#[test]
fn config_file_overrides_profile() {
    let config = tempfile::NamedTempFile::with_suffix(".toml").unwrap();
    fs::write(
        config.path(),
        "profile = \"gfm\"\n[render]\nhr-style = \"underscores\"\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--config", config.path().to_str().unwrap()])
        .arg(fixture_dir().join("table.html"));
    cmd.assert()
        .success()
        .stdout(contains("| Language | Type        |"));
}

#[test]
fn invalid_config_selector_fails() {
    let config = tempfile::NamedTempFile::with_suffix(".toml").unwrap();
    fs::write(
        config.path(),
        "[cleanup]\nremove-selectors = [\"<<<bad\"]\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--config", config.path().to_str().unwrap()])
        .arg(fixture_dir().join("basic.html"));
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("invalid selector"));
}

#[test]
fn batch_output_dir() {
    let out_dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("--output-dir")
        .arg(out_dir.path())
        .arg(fixture_dir().join("basic.html"))
        .arg(fixture_dir().join("table.html"));
    cmd.assert().success();

    let basic = out_dir.path().join("basic.md");
    let table = out_dir.path().join("table.md");
    assert!(basic.exists());
    assert!(table.exists());
    assert!(fs::read_to_string(basic).unwrap().contains("# Hello World"));
    assert!(
        fs::read_to_string(table)
            .unwrap()
            .contains("| Language | Type        |")
    );
}

#[test]
fn mirror_recursive_directory() {
    let root = tempfile::tempdir().unwrap();
    let sub = root.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("page.html"), "<h1>Nested</h1>").unwrap();

    let out_dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("--recursive")
        .arg("--mirror")
        .arg("--output-dir")
        .arg(out_dir.path())
        .arg(&sub);
    cmd.assert().success();

    let mirrored = out_dir.path().join("page.md");
    assert!(mirrored.exists(), "expected {}", mirrored.display());
    assert!(fs::read_to_string(&mirrored).unwrap().contains("# Nested"));
}

#[test]
fn output_policy_skip_existing() {
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("basic.md");
    fs::write(&out, "existing").unwrap();

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("--output-dir")
        .arg(out_dir.path())
        .arg("--output-policy")
        .arg("skip-existing")
        .arg(fixture_dir().join("basic.html"));
    cmd.assert().success();

    assert_eq!(fs::read_to_string(&out).unwrap(), "existing");
}

#[test]
fn output_policy_fail_if_exists() {
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("basic.md");
    fs::write(&out, "existing").unwrap();

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("--output-dir")
        .arg(out_dir.path())
        .arg("--output-policy")
        .arg("fail-if-exists")
        .arg(fixture_dir().join("basic.html"));
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("already exists"));
}

#[test]
fn atomic_write() {
    let out_dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("--atomic")
        .arg("--output-dir")
        .arg(out_dir.path())
        .arg(fixture_dir().join("basic.html"));
    cmd.assert().success();

    let out = out_dir.path().join("basic.md");
    assert!(out.exists());
    assert!(fs::read_to_string(&out).unwrap().contains("# Hello World"));
}

#[test]
fn manifest_written() {
    let out_dir = tempfile::tempdir().unwrap();
    let manifest = out_dir.path().join("manifest.json");

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("--output-dir")
        .arg(out_dir.path())
        .arg("--manifest")
        .arg(&manifest)
        .arg(fixture_dir().join("basic.html"));
    cmd.assert().success();

    assert!(manifest.exists());
    let content = fs::read_to_string(&manifest).unwrap();
    assert!(content.contains("input_hash"));
    assert!(content.contains("output_hash"));
}

#[test]
fn check_mode_passes_when_unchanged() {
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("basic.md");

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("-o")
        .arg(&out)
        .arg(fixture_dir().join("basic.html"));
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("--check")
        .arg("-o")
        .arg(&out)
        .arg(fixture_dir().join("basic.html"));
    cmd.assert().success();
}

#[test]
fn check_mode_fails_when_changed() {
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("basic.md");
    fs::write(&out, "different").unwrap();

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("--check")
        .arg("-o")
        .arg(&out)
        .arg(fixture_dir().join("basic.html"));
    cmd.assert().failure().code(2).stderr(contains("changed"));
}

#[test]
fn encoding_override() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("latin.html");
    fs::write(&input, b"<p>caf\xe9</p>").unwrap();

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("--encoding").arg("windows-1252").arg(&input);
    cmd.assert().success().stdout(contains("café"));
}

#[test]
fn dry_run_does_not_write() {
    let out_dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.arg("--dry-run")
        .arg("--output-dir")
        .arg(out_dir.path())
        .arg(fixture_dir().join("basic.html"));
    cmd.assert().success();
    assert!(out_dir.path().read_dir().unwrap().next().is_none());
}

#[test]
fn profile_extended() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--profile", "extended"])
        .arg(fixture_dir().join("extended.html"));
    cmd.assert().success().stdout(contains("==important=="));
}

#[test]
fn profile_obsidian_frontmatter_and_wikilink() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("obsidian.html");
    fs::write(
        &input,
        "<!doctype html><html><head>\
         <title>My Note</title>\
         <meta name=\"description\" content=\"A note\">\
         </head><body>\
         <p><a class=\"wikilink\" href=\"Another page\">another note</a></p>\
         </body></html>",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args([
        "--profile",
        "obsidian",
        "--metadata-title",
        "--metadata-description",
    ])
    .arg(&input);
    cmd.assert()
        .success()
        .stdout(contains("---"))
        .stdout(contains("title: My Note"))
        .stdout(contains("description: A note"))
        .stdout(contains("[[Another page|another note]]"));
}

#[test]
fn profile_plain_text() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--profile", "plain-text"])
        .arg(fixture_dir().join("basic.html"));
    cmd.assert()
        .success()
        .stdout(contains("# Hello World").not())
        .stdout(contains("Hello World"));
}

#[test]
fn metadata_flags_extract_title() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--metadata-title", "--metadata-description"])
        .arg(fixture_dir().join("metadata.html"));
    cmd.assert().success();
}

#[test]
fn link_style_reference() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--link-style", "reference"])
        .arg(fixture_dir().join("links.html"));
    cmd.assert()
        .success()
        .stdout(contains("[Relative link][1]"))
        .stdout(contains("[1]: /page"));
}

#[test]
fn reference_placement_adjacent() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args([
        "--link-style",
        "reference",
        "--reference-placement",
        "adjacent",
    ])
    .arg(fixture_dir().join("links.html"));
    cmd.assert()
        .success()
        .stdout(contains("[ref1]: /page"))
        .stdout(contains("[ref2]: https://example.com/page"));
}

#[test]
fn reference_placement_section_end() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("refs.html");
    fs::write(
        &input,
        "<h1>Section A</h1><p><a href=\"/a\">link a</a></p>\
         <h2>Section B</h2><p><a href=\"/b\">link b</a></p>",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args([
        "--link-style",
        "reference",
        "--reference-placement",
        "section-end",
    ])
    .arg(&input);
    cmd.assert()
        .success()
        .stdout(contains("[ref1]: /a"))
        .stdout(contains("[ref2]: /b"));
}

#[test]
fn image_mode_reference() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--image-mode", "reference"])
        .arg(fixture_dir().join("image_mode.html"));
    cmd.assert()
        .success()
        .stdout(contains("![Photo][img1]"))
        .stdout(contains("[img1]: a.jpg"))
        .stdout(contains("[img2]: b.png"));
}

#[test]
fn image_mode_skip() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--image-mode", "skip"])
        .arg(fixture_dir().join("image_mode.html"));
    cmd.assert()
        .success()
        .stdout(contains("Photo").not())
        .stdout(contains("Diagram").not());
}

#[test]
fn image_mode_alt_text() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--image-mode", "alt-text"])
        .arg(fixture_dir().join("image_mode.html"));
    cmd.assert()
        .success()
        .stdout(contains("Photo"))
        .stdout(contains("Diagram"))
        .stdout(contains("![").not());
}

#[test]
fn profile_pandoc_preserves_raw_html() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--profile", "pandoc"])
        .arg(fixture_dir().join("extended.html"));
    cmd.assert()
        .success()
        .stdout(contains("==important=="))
        .stdout(contains("<div class=\"footnotes\">"));
}

#[test]
fn profile_mdx_safe_escapes_braces() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("mdx.html");
    fs::write(&input, "<p>Hello {world}</p>").unwrap();

    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args(["--profile", "mdx-safe"]).arg(&input);
    cmd.assert().success().stdout(contains(r#"\{world\}"#));
}

#[test]
fn metadata_canonical_url_flag() {
    let mut cmd = Command::cargo_bin("htmlmd").unwrap();
    cmd.args([
        "--profile",
        "obsidian",
        "--metadata-title",
        "--metadata-description",
        "--metadata-canonical-url",
    ])
    .arg(fixture_dir().join("metadata.html"));
    cmd.assert()
        .success()
        .stdout(contains("title: Page Title"))
        .stdout(contains("description: Page description"))
        .stdout(contains("canonical_url: https://example.com/page"));
}
