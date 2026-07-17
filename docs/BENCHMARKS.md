# Benchmarks

Tracking file for the performance work in [`ROADMAP.md`](ROADMAP.md). Every
optimization milestone records before/after numbers here. No perf claim ships
without a row in this file.

## Cross-tool comparison (M4)

Harness: [`benches/compare/`](../benches/compare/) — hyperfine over full CLI
invocations (process + interpreter startup included, which is what a
command-line user actually pays), every tool converting **byte-identical
input** from the shared synthetic corpus, each asked for GFM-style output
where it has a flag for it. Reproduce with `benches/compare/run.sh`.

Raspberry Pi 5 (aarch64), quiet machine, 2026-07-16. Mean ± σ; the
parenthesised figure is slowdown relative to htmlmd (lower is better for
them):

| tool | wiki (1.0 MB) | news (217 KB) | docs (245 KB) | tables (211 KB) |
|---|---|---|---|---|
| **htmlmd** 0.1.0 | **59±1 ms** | **17±1 ms** | **30±3 ms** | **30±1 ms** |
| [html-to-markdown v2] (Go) 2.5.2 | 174±13 ms (2.9×) | 34±2 ms (2.0×) | **23±1 ms (0.8×)** | 59±2 ms (1.9×) |
| [turndown] (Node) 7.2 + gfm | 1482±34 ms (25.1×) | 321±20 ms (18.7×) | 412±2 ms (13.8×) | 304±4 ms (10.0×) |
| [markdownify] (Python) 1.2 | 1164±27 ms (19.7×) | 307±2 ms (17.9×) | 286±3 ms (9.6×) | 769±24 ms (25.3×) |
| [pandoc] 3.10 | 5761±607 ms (97.5×) | 1026±17 ms (59.8×) | 883±11 ms (29.6×) | 1536±16 ms (50.5×) |

**Honest reading, including where we lose.** Against the interpreted
converters (turndown, markdownify) htmlmd is an order of magnitude faster —
10–25× — and pandoc, the only other tool with multi-flavor output, is 30–98×
slower. The one genuine competitor is Go's html-to-markdown v2: htmlmd is
2–3× faster on prose- and table-heavy pages, **but v2 is ~25% faster on the
code-heavy `docs` corpus**. That is not noise and not a mystery: on that
corpus htmlmd runs code-language detection (class parsing plus a content
heuristic) that v2 simply does not implement, so it is doing strictly more
work per document. Turn `semantic.detect-languages` off and the gap closes —
but the honest default-vs-default number is the one in the table.

Speed is not the whole comparison: see the profile/feature matrix in the
README for what these tools do and don't convert. This harness measures time
only.

### Windows x86_64

Same corpus, same rules, run with [`benches/compare/windows/run.ps1`](../benches/compare/windows/run.ps1).
**These numbers share no table with the Pi figures above** — different
architecture, different OS, different process-spawn cost. Compare tools
within this table only.

<!-- TODO: fill in CPU model before publishing -->
Windows 11, x86_64 (CPU: _TBD_), quiet machine, 2026-07-17. hyperfine 1.20.0,
mean ± σ; parenthesised figure is slowdown relative to htmlmd:

| tool | wiki (1.0 MB) | news (217 KB) | docs (245 KB) | tables (211 KB) |
|---|---|---|---|---|
| **htmlmd** 0.1.0 | **34±5 ms** | **19±4 ms** | **24±6 ms** | **24±6 ms** |
| [html-to-markdown v2] (Go) 2.5.2 | 62±3 ms (1.8×) | 20±3 ms (1.0×) | **19±6 ms (0.8×)** | 28±5 ms (1.2×) |
| [turndown] (Node) 7.2 + gfm | 690±3 ms (20.4×) | 160±4 ms (8.4×) | 207±4 ms (8.6×) | 144±9 ms (6.1×) |
| [markdownify] (Python) 1.2 | 575±10 ms (17.0×) | 175±7 ms (9.2×) | 165±4 ms (6.8×) | 382±7 ms (16.2×) |
| [pandoc] 3.10 | 1284±13 ms (38.0×) | 282±7 ms (14.8×) | 243±5 ms (10.0×) | 393±5 ms (16.7×) |

**Reading these honestly — most of this table is at the measurement floor.**
Only `wiki` (1.0 MB) is large enough that conversion work dominates process
startup; the other three corpora are 211–245 KB and every fast tool lands
near 20 ms regardless of what it does. The tell is in the dispersion: the
interpreted tools have a coefficient of variation of 0.4–6%, while htmlmd
and html-to-markdown sit at 13–31% with maxima 2–2.4× their own medians.
That spread is Windows process-spawn jitter, not workload variance — a
noisy machine would have hit pandoc's 1.3 s run hardest, and it did not
touch it at all. hyperfine's "re-run on a quiet PC" warning fires on this
data for that reason and should not be read as a machine problem.

So: the 6–38× gaps over turndown, markdownify and pandoc are real and far
exceed the floor. The htmlmd-vs-html-to-markdown comparison is only
trustworthy on `wiki` (1.9× our favour, and mean/median/min all agree). On
the three small corpora, treat it as a tie.

The `docs` result reproduces the Pi finding exactly — v2 is ~25% faster
there on both platforms, for the same reason: htmlmd runs code-language
detection that v2 does not implement. Two independent architectures showing
the same ratio is a good sign the explanation is right.

### Rust libraries, in-process (no CLI startup)

From the criterion bench (`cargo bench -p htmlmd-core --bench convert_bench`),
same corpus, median of 10 samples. These are library calls, so process
startup is excluded and the comparison is architecture-vs-architecture:

| library | wiki | news | docs | tables | what it does |
|---|---|---|---|---|---|
| **htmlmd** (commonmark) | 44.2 ms | 11.4 ms | 14.2 ms | 24.6 ms | full pipeline: 7 profiles, selector rules, cleanup, metadata, limits |
| `htmd` 0.5.4 (our renderer basis) | 38.3 ms | 9.2 ms | 10.3 ms | 20.2 ms | tag translation only |
| [`fast_html2md`] 0.0.62 | **29.0 ms** | **5.9 ms** | **5.8 ms** | **15.0 ms** | single flavor, streaming rewriter |
| [`mdka`] 2.1.6 | 55.1 ms | 12.2 ms | 13.1 ms | 19.7 ms | single flavor, 5 modes |

**We are not the fastest Rust HTML→Markdown library, and shouldn't claim to
be.** `fast_html2md` is 1.5–2.4× faster than htmlmd — and faster than `htmd`
itself — because it is architecturally different: it streams through
Cloudflare's `lol_html` rewriter and never builds a DOM. htmlmd builds a
tree because profiles, CSS-selector rules, table-complexity analysis, and
metadata extraction all require random access to the document; that tree is
the price of the feature set, and M3.5 got its cost down to 8–19% over a
tree-based renderer doing none of that work.

The accurate claim is narrower and still worth making: **htmlmd is the
fastest converter that offers profiles, selector rules, and metadata** — it
beats `mdka` outright, and every non-Rust tool by 2–98×. If you need only
plain single-flavor conversion at maximum speed, `fast_html2md` is the better
tool and we should say so.

[`fast_html2md`]: https://github.com/spider-rs/html2md
[`mdka`]: https://github.com/nabbisen/mdka-rs

[html-to-markdown v2]: https://github.com/JohannesKaufmann/html-to-markdown
[turndown]: https://github.com/mixmark-io/turndown
[markdownify]: https://github.com/matthewwithanm/python-markdownify
[pandoc]: https://github.com/jgm/pandoc

## Method

- Harness: `cargo bench -p htmlmd-core --bench convert_bench`
  (Criterion; corpus groups use 10 samples × 4 s measurement).
- Corpus: deterministic synthetic documents generated by the bench itself —
  `wiki` (Wikipedia-style prose), `news` (boilerplate + tracking params +
  srcset images), `docs` (code-block heavy), `tables` (incl. complex
  rowspan/colspan tables). Sizes are printed at bench start.
- `raw-htmd` rows convert the same input with the underlying `htmd` library
  and no htmlmd pipeline: the "minimal library" baseline. The htmlmd/raw-htmd
  ratio is the wrapper overhead tracked by ROADMAP M3.

## Machine

- Raspberry Pi 5 (BCM2712, aarch64-unknown-linux-gnu), Linux 6.12, Rust stable.
- Numbers below are medians reported by Criterion. Treat cross-machine
  comparisons as invalid; only same-machine deltas matter.

## Results

### M0 baseline — commit bb4a388 (pre-optimization), 2026-07-16

Corpus sizes: wiki 1,044,281 B · news 217,498 B · docs 244,839 B · tables 210,770 B.

| Benchmark | commonmark | gfm | extended | plain-text | raw htmd | overhead vs htmd |
|---|---|---|---|---|---|---|
| corpus/wiki | 978.7 ms | 972.4 ms | 987.6 ms | 994.9 ms | 44.5 ms | **22.0×** |
| corpus/news | 24.3 ms | 24.1 ms | 24.7 ms | 26.4 ms | 9.9 ms | **2.4×** |
| corpus/docs | 554.1 ms | 557.1 ms | 563.8 ms | 559.9 ms | 11.2 ms | **49.6×** |
| corpus/tables | 58.3 ms | 58.3 ms | 59.8 ms | 60.7 ms | 26.2 ms | **2.2×** |

Fixtures: basic 105 µs · table 112 µs · malformed 84 µs.

Reading: the overhead explodes exactly where per-element regex compilation
runs — `docs` (code-heavy, ~50×) and `wiki` (inline `<code>` in every
paragraph, 22×) — while `news`/`tables` sit near the structural ~2× cost of
the double parse. This is what ROADMAP M1 targets.

### M1 — regex/selector caching, thin LTO + codegen-units=1, mimalloc (2026-07-16)

Changes: process-wide cache for user-configured regexes (`regex_cache.rs`),
Lazy statics for fixed table selectors and `strip_markdown` regexes,
`CompiledRewriteRules` resolved once per conversion, `[profile.release]`
tuning, mimalloc as the binaries' default allocator (bench uses it too).
An intermediate version compiled user patterns per *conversion*; that
regressed no-code documents 4–6× (fixture/basic 105→460 µs) and was replaced
by the process-wide cache before landing.

| Benchmark | M0 (commonmark) | M1 (commonmark) | speedup | raw htmd M1 | overhead now |
|---|---|---|---|---|---|
| corpus/wiki | 978.7 ms | 109.0 ms | **9.0×** | 36.9 ms | 2.9× |
| corpus/news | 24.3 ms | 22.0 ms | 1.1× | 8.7 ms | 2.5× |
| corpus/docs | 554.1 ms | 28.5 ms | **19.5×** | 9.9 ms | 2.9× |
| corpus/tables | 58.3 ms | 47.2 ms | 1.2× | 19.3 ms | 2.4× |

Fixtures: basic 87 µs (was 105) · table 84 µs (was 112) · malformed 65 µs
(was 84). Other profiles track commonmark within a few percent; raw htmd
itself gained ~15–35% from LTO + mimalloc, so the speedup columns understate
the absolute improvement.

Reading: the pathological regex-compilation overhead is gone; every corpus
now sits at a uniform **~2.4–2.9×** over raw htmd. That remaining band is the
structural double-parse + serialize cost — precisely ROADMAP M3's target
(single-parse pipeline, goal ≤ ~1.3×).

### M3 — single-parse native renderer (2026-07-16)

`convert()` now renders directly from the cleaned scraper DOM via the native
walker (ported from htmd, byte-identical output — 26 differential parity
suites). The serialize + re-parse step is gone; `keep-only` prunes the live
tree instead of triggering a third parse.

| Benchmark | M1 (htmd pipeline) | M3 native | speedup | raw htmd | overhead now |
|---|---|---|---|---|---|
| corpus/wiki | 128.0 ms* | **77.7 ms** | 1.65× | 37.5 ms | 2.07× |
| corpus/news | 22.7 ms* | **12.8 ms** | 1.78× | 8.7 ms | 1.46× |
| corpus/docs | 28.7 ms* | **15.6 ms** | 1.84× | 9.9 ms | 1.58× |
| corpus/tables | 47.8 ms* | **31.0 ms** | 1.54× | 19.8 ms | 1.57× |

\* the `htmd-backend` rows measured in the same M3 run (the old pipeline kept
behind the `backend-htmd` feature), so the comparison is same-commit,
same-machine. Fixtures: basic 62 µs · table 63 µs · malformed 50 µs.

Cumulative since M0 baseline: **wiki 978.7 → 77.7 ms (12.6×), docs 554.1 →
15.6 ms (35×)**, news 24.3 → 12.8 ms, tables 58.3 → 31.0 ms.

Honest scorecard vs the ≤1.3× goal: 1.46–1.58× on three corpora, 2.07× on
the 1 MB wiki doc. The residual is no longer parsing — it's the ~15
sequential cleanup passes each scanning the tree with selectors (work raw
htmd doesn't do at all). Closing further means fusing cleanup passes into
fewer traversals — tracked as a ROADMAP follow-up, with per-pass profiling
before any rewrite.

### M3.5 — cleanup-pass fusion (2026-07-16) — ≤1.3× goal met

Per-pass profiling (the `pass_timing` ignored test) showed cleanup cost
35.6 ms on a wiki-scale document: code-language detection ran its regex
battery on thousands of *inline* code spans (11.1 ms) where a language class
has zero output effect; hidden-content removal used six selector scans
(7.0 ms); remove-tags eight more (5.9 ms); URL rewriting three plus a title
scan (6.9 ms). Fixes: one fused removal walk, detection restricted to
`pre code`, one classification walk feeding all per-element passes
(details/forms/media/custom-elements/tables/images), a fused URL+title
attribute walk, and a no-query fast path in tracking-param stripping.
Cleanup now totals **~7.0 ms** on the same document.

| Benchmark | M3 | M3.5 | raw htmd | **overhead** |
|---|---|---|---|---|
| corpus/wiki | 77.7 ms | **42.1 ms** | 38.1 ms | **1.10×** |
| corpus/news | 12.8 ms | **10.9 ms** | 9.2 ms | **1.18×** |
| corpus/docs | 15.6 ms | **12.6 ms** | 11.7 ms | **1.08×** |
| corpus/tables | 31.0 ms | **23.2 ms** | 19.5 ms | **1.19×** |

Cumulative since the M0 baseline (same machine, same corpus): **wiki
978.7 → 42.1 ms (23×), docs 554.1 → 12.6 ms (44×)**, news 2.2×, tables 2.5×.
The full pipeline — cleanup, tracking-param stripping, language detection,
safety limits, metadata — now costs 8–19% over the bare-bones library.

One deliberate behavior change: query-less URLs are no longer round-tripped
through `url::Url`, so `https://example.com` stays as written instead of
gaining a trailing slash. Three expectations updated accordingly
(`expected/basic.md` and two inline assertions) — the only expected-output
edits in the entire M3 line, each a pure URL-spelling diff.
