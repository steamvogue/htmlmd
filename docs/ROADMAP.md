# Performance & quality roadmap

Goal: make `htmlmd` the **fastest full-featured** HTML→Markdown converter on
the market — measurably faster than Go `html-to-markdown` v2, pandoc,
turndown, and markdownify, and within noise of the fastest minimal Rust
libraries (raw `htmd`, `fast_html2md`) while doing far more work — and prove
it with published, reproducible benchmarks.

Honest framing: a minimal single-flavor library will always have less work to
do. The winnable crown is "fastest converter with profiles, selector rules,
cleanup, and metadata," plus parity with minimal libraries on plain documents.

Milestones are ordered so each one ships alone. Do not start M3 before M0
exists — every perf claim needs a before/after number.

---

## M0 — Measure first (½ day) — ✅ done 2026-07-16

Nothing gets optimized before it's measured.

1. **Benchmark corpus.** Add `benches/corpus/` with 4 realistic inputs:
   a large Wikipedia article (~1.4 MB), a news article with heavy tracking
   markup, an API-docs page (code-block heavy), and a table-heavy page.
2. **Extend `crates/htmlmd-core/benches/convert_bench.rs`** to run each
   corpus file × each profile, plus a "raw htmd" baseline (htmd is already a
   dependency) so the wrapper overhead is a tracked number, not a guess.
3. **Record baseline numbers** in `docs/BENCHMARKS.md` (machine, commit,
   table of results). Update this file at every milestone.
4. **CI**: add `cargo fmt --check`, `cargo clippy -- -D warnings`, and
   `Swatinem/rust-cache` to `.github/workflows/ci.yml`. Run `cargo deny check`
   (a `deny.toml` already exists but nothing runs it).

Acceptance: `cargo bench` produces a wrapper-overhead-vs-htmd ratio.

## M1 — Quick perf wins, no API changes — ✅ done 2026-07-16 (measured: 9–19.5× on regex-heavy corpus, 1.1–1.3× elsewhere; see BENCHMARKS.md)

1. **Cache fixed selectors/regexes** in `once_cell::sync::Lazy` statics:
   - `Selector::parse("tr")`, `("td, th")`, `("table table")`,
     `("[rowspan], [colspan]")` recompiled per table/row —
     `cleanup.rs:965–998`.
   - Language-class regexes recompiled per `<code>` element —
     `cleanup.rs:469–483`.
   - `strip_markdown`'s ~11 regexes compiled per conversion — `lib.rs:150–180`.
   - Template regex `\{attr:(\w+)\}` per matched element —
     `htmd_handlers.rs:1052`.
2. **Compile user-supplied patterns once.** `options/validation.rs:60–77`
   already compiles every user regex/selector to validate it, then throws the
   result away; runtime recompiles per URL (`rewrite.rs:49–53`) and silently
   skips on error. Store compiled forms in a `CompiledOptions` (built once per
   conversion) and use them everywhere. Kills both the recompile cost and the
   silent-skip behavior.
3. **Add a `[profile.release]` section** to the workspace `Cargo.toml`
   (currently absent — cargo defaults leave real speed on the table):
   `lto = "thin"`, `codegen-units = 1`, `strip = true`. (Fat LTO is worth
   an experiment in release CI; `panic = "abort"` is deliberately avoided so
   a panicking server handler can't take down the whole process.)
4. **Optional allocator**: `mimalloc` behind a default-on feature in the CLI
   and server crates (alloc-heavy DOM workloads typically gain 10–20%).
5. **Replace `Arc<Mutex<ReferenceState>>` with `Rc<RefCell<…>>`**
   (`htmd_handlers.rs:803–989`) if htmd's handler signature allows single-
   threaded state; otherwise keep `Mutex` but replace `.lock().unwrap()` with
   poison-recovery (`unwrap_or_else(PoisonError::into_inner)`). Removes both
   lock overhead and the latent library panic.

Acceptance: benchmark deltas recorded in `BENCHMARKS.md`; no fixture changes.

## M2 — Correctness, robustness, honest options — ✅ done 2026-07-16

All ten items landed. Notes: the plain-text rework protects htmd's
backslash-escapes via a private-use-area encoding and strips only *paired*
markers; custom rules are unified by renaming claimed elements to an
`htmlmdrule` marker tag in the DOM pass (full CSS selectors everywhere, one
descending priority order); 20 inert option fields and 14 orphaned enums were
deleted while `task-lists` (previously advertised but entirely unimplemented)
and `heading-offset` were wired for real; the server gained bind/body-limit
config, graceful shutdown, and its first tests.

1. **Plain-text profile rework.** `strip_markdown` (`lib.rs:150–180`) deletes
   `*`, `_`, `~`, `^`, `==`, `++` from legitimate prose and code. Replace
   character-stripping with a plain-text handler set at render time. Add
   regression fixtures containing literal `a*b`, `x^2`, `snake_case`, `C++`.
2. **YAML frontmatter escaping.** `build_obsidian_frontmatter`
   (`lib.rs:128–143`) must emit quoted/escaped scalars (titles containing
   `:`, `#`, or newlines currently produce invalid YAML).
3. **Real MDX safety.** `escape_mdx` (`lib.rs:145–148`) only escapes braces;
   handle `<`/`>` in text runs and HTML comments too, or rename the option
   honestly.
4. **Unify the two custom-rule paths.** DOM path (`cleanup.rs:863–927`) and
   handler path (`htmd_handlers.rs:1005–1206`) disagree on priority order
   (descending vs ascending) and selector power (full scraper selectors vs
   hand-rolled tag/class/id matcher). Fix: match *all* selectors in the DOM
   pass with scraper, tag matched nodes with `data-htmlmd-rule="<idx>"`
   markers (the table pipeline already uses this pattern), and make handlers
   act on markers. One priority order, full selector support everywhere.
5. **Deduplicate** `choose_largest_srcset` (`rewrite.rs:84` vs
   `cleanup.rs:557`) and the near-identical reference link/image handler
   bodies (`htmd_handlers.rs:903–916` vs `961–974`).
6. **Single limits enforcement point.** `lib.rs:47–53` hard-errors on input
   size while `cleanup.rs:75–85` warns for the same limit — the second branch
   is unreachable. Enforce all limits in one place with one strict/lenient
   policy.
7. **Wire or remove the ~20 inert options** (`options.rs:9–14` admits they do
   nothing). Wire the cheap ones: `ordered-list-marker`, `emphasis-marker`,
   `strong-marker`, `heading-offset`, `list-indent`, `code-fence-min-length`.
   Delete the rest from the schema (better a smaller honest surface than a
   silently-ignored one) and update `OPTION_REFERENCE.md`. `smart-punctuation`
   must either work or stop being set by the pandoc profile.
8. **Server hardening** (`crates/htmlmd-server/src/main.rs`): `--bind`/`PORT`
   config instead of hardcoded `127.0.0.1:3000`, no `.unwrap()` on
   bind/serve, request body size limit, graceful shutdown, and axum
   integration tests (it currently has zero).
9. **Wire `--verbose`** (`cli.rs:165`) to the tracing filter or remove it.
10. Fix `total_input_count` (`convert.rs:57–78`) counting a directory as one
    input, which defeats the `-o`-with-multiple-inputs guard.

Acceptance: all fixtures pass; new regression fixtures for 1–3; server tests.

## M3 — Single-parse pipeline — ✅ done 2026-07-16 (measured: 1.54–1.84× vs M1; overhead vs raw htmd now 1.46–2.07× — see honest scorecard in BENCHMARKS.md; remaining gap is cleanup-pass cost, tracked below)

Today every conversion parses HTML **twice** (scraper for cleanup, then
htmd's rcdom after a full string re-serialization at `cleanup.rs:52`), and
`keep-only` parses a third time (`cleanup.rs:268`). This is the dominant
structural cost and the last thing between htmlmd and the minimal libraries.

1. **Write a native renderer** that walks the already-cleaned scraper
   `ego-tree` DOM directly and emits Markdown — port the logic of
   `htmd_handlers.rs` (semantic tags, tables, footnotes, math, mermaid,
   alerts, references, custom rules) to tree-walk visitors. Parse once,
   mutate once, render once. No intermediate HTML string, no `rcdom`.
2. **Keep `ConverterBackend`** and ship the native renderer as the default
   backend; keep `HtmdBackend` behind a `backend-htmd` feature during the
   transition. This also fixes the current leak where custom backends
   silently lose every feature.
3. **Golden parity**: the fixture suite is the contract. Run both backends in
   CI until the native renderer matches on every fixture, then flip the
   default.
4. Fix `keep-only` to prune the existing tree instead of re-parsing
   (`cleanup.rs:268`). ✅
5. ~~Make `convert_to_writer` actually stream~~ **Deferred indefinitely with
   rationale**: block-structured rendering, reference-definition placement,
   and profile post-processing (frontmatter, plain-text strip, MDX escape)
   all require the full document before the first byte is final. A streaming
   mode would cover only a trivial subset dishonestly; revisit only if a
   chunked-input use case materializes.

Acceptance: `BENCHMARKS.md` shows wrapper-overhead-vs-htmd ratio ≤ ~1.3× on
the plain corpus; memory high-water mark roughly halved on the 1.4 MB page.

## M3.5 — Cleanup-pass fusion — ✅ done 2026-07-16 (measured: cleanup 35.6→7.0 ms; overhead vs raw htmd now **1.08–1.19×**, goal met)

The post-M3 overhead vs raw htmd (1.46–2.07×) is dominated by `clean_html`'s
~15 sequential DOM passes, each walking the tree with selector scans.
Profile per-pass cost first (criterion bench on `clean_html_to_dom` alone),
then fuse compatible passes into shared traversals and skip passes whose
options are at no-op defaults. Target: overhead ≤ ~1.3× on all corpora.
Parity contract: the differential suite must stay byte-identical.

## M4 — Prove it publicly (2–3 days)

1. **Cross-tool benchmark harness** (`benches/compare/`): scripted runs of
   htmlmd vs raw htmd, fast_html2md, mdka, Go html-to-markdown v2, turndown
   (node), markdownify (python), pandoc on the same corpus. Publish method +
   results in `docs/BENCHMARKS.md`; link from README.
2. **Fuzzing**: `cargo-fuzz` target on `convert` (html5ever makes crashes
   unlikely; the fuzzer is for the cleanup/renderer logic and limits).
   Property tests: output determinism, idempotent re-conversion, limits
   always honored.
3. **Release automation**: turn the example matrix in `BUILD_AND_DEPLOY.md`
   into a real `.github/workflows/release.yml` — tag-triggered, builds
   linux x86_64/aarch64 (+musl), macOS aarch64, Windows x86_64, uploads
   `dist/<triple>/` layout with `SHA256SUMS` as release assets; multi-arch
   Docker image via buildx.
4. **Publish to crates.io** (`htmlmd-core` then `htmlmd-cli`): add
   `keywords`, `categories`, `readme` to each crate's `Cargo.toml`;
   `cargo-binstall` metadata for prebuilt installs.

## M5 — Cosmetic / housekeeping (can run anytime)

- ~~`LICENSE-MIT` + `LICENSE-APACHE` files~~ (done — dual license per the
  existing `MIT OR Apache-2.0` declarations).
- README badges (CI, crates.io, docs.rs, license) once public.
- `CHANGELOG.md` (Keep a Changelog format) and `CONTRIBUTING.md`.
- Retire the "Phase 1/2/3" comments in source (`cleanup.rs:437,462,716,861`,
  `rewrite.rs:9`, `options.rs:9–14,621`) — the phase model is done; comments
  should state what the code does, not project history.
- Remove the unused `parallel`/`rayon` optional dependency from
  `htmlmd-core` (only the CLI uses rayon).
- Decide the fate of `MathOutput::BlockDollar` (currently byte-identical to
  `InlineDollar`, `htmd_handlers.rs:412–425`) and `HeadingStyle::Keep`
  (silently maps to Atx).
