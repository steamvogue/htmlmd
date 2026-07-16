# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Everything below is pre-1.0 groundwork: the option schema is still allowed to
change, and several options were removed outright rather than shipped as
placeholders. See [`docs/ROADMAP.md`](docs/ROADMAP.md) for the plan and
[`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) for measured numbers behind every
performance claim.

### Added

- Cross-tool benchmark harness ([`benches/compare/`](benches/compare/)) comparing
  `htmlmd` against turndown, markdownify, pandoc, and Go's html-to-markdown v2
  over identical input, plus in-process criterion rows for `htmd`,
  `fast_html2md`, and `mdka`.
- Property and robustness test suite (`crates/htmlmd-core/tests/properties.rs`):
  determinism, no-panic on arbitrary and adversarial input, profile matrix,
  strict limits, re-conversion stability, garbage sweep.
- Differential parity suite (`crates/htmlmd-core/tests/native_parity.rs`):
  the native renderer must be byte-identical to the previous engine across
  every profile and fixture.
- Tag-triggered release workflow: binaries for five targets with `SHA256SUMS`,
  plus a multi-arch `ghcr.io` server image. `cargo-binstall` metadata.
- `ConversionOptions::commonmark()`, mirroring the other profile constructors.
- `--verbose` now reports per-file conversion diagnostics.
- `LICENSE-MIT` / `LICENSE-APACHE`, `CONTRIBUTING.md`, `CHANGELOG.md`.

### Changed

- **Single-parse pipeline.** Conversion parses HTML once (scraper), cleans that
  DOM, and renders Markdown directly from it. The previous engine re-parsed the
  serialized output; it remains behind the `backend-htmd` feature for parity
  testing.
- **Performance** (Raspberry Pi 5, synthetic corpus): 1 MB article
  978.7 ms → 42.1 ms (23×); code-heavy page 554.1 ms → 12.6 ms (44×). Overhead
  over the bare `htmd` renderer is now 8–19%.
- **Plain-text profile is no longer lossy**: literal `*`, `_`, `^`, `++` in
  prose and code survive; code-block content is kept.
- Obsidian frontmatter values are emitted as quoted YAML scalars.
- Custom rules accept full CSS selectors for every action (class-only selectors
  were previously ignored for template/link/image rules) under one priority order.
- The HTTP server takes `--bind`/`HTMLMD_BIND` and `HTMLMD_MAX_BODY_BYTES`,
  shuts down gracefully, and no longer unwraps on bind failure.
- Release binaries are written to `dist/<target-triple>/` by
  `scripts/build-release.sh`.
- `math.output = "block-dollar"` now emits `$$…$$` for inline math too; it was
  previously identical to `inline-dollar`.

### Fixed

- **Stack overflow (process abort) on deeply nested HTML.** `Limits` derived
  `Default`, so every limit defaulted to unlimited while the renderer recursed
  per DOM level; `~1900` nested elements aborted the process — reachable through
  the HTTP server. `max-dom-depth` now defaults to 256 and is enforced:
  over-deep subtrees are pruned with a diagnostic (rejected under `strict`).
- Mutex poisoning in the reference-link handlers could panic later conversions.
- `{attr:…}` template placeholders now match hyphenated attribute names.
- `-o` with multiple inputs is rejected based on resolved jobs, so a directory
  or glob expanding to many files no longer slips through.
- GFM task lists (`- [x]`) are actually converted; they were advertised but
  unimplemented.

### Removed

- 20 option fields and 14 enums that were parsed and documented but never
  affected output. Configuration files containing them still load — unknown
  keys are ignored.
- `heading-style = "keep"` (silently behaved as `atx`) and
  `form-handling = "checklist"` (did nothing).
- The unused `parallel` feature on `htmlmd-core` (only the CLI uses rayon).

### Known limitations

- Parsing is quadratic in nesting depth, inherited from `html5ever`. Bound
  `max-input-bytes` for untrusted input; see
  [`docs/API_AND_WEB_SERVICE.md`](docs/API_AND_WEB_SERVICE.md).
- `convert_to_writer` buffers rather than streams: reference-definition
  placement and profile post-processing need the whole document.
- As a library, `fast_html2md` is faster for plain single-flavor conversion —
  it streams instead of building a DOM. See `docs/BENCHMARKS.md`.
