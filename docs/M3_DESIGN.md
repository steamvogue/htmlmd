# M3 design: single-parse pipeline (native renderer)

Working notes for ROADMAP M3. Kept in-repo so the design survives across
sessions and agents.

## Problem

Every conversion parses HTML twice: scraper (cleanup DOM passes) → serialize
to string (`cleanup.rs` `document.html()`) → htmd re-parses with
`markup5ever_rcdom` → handler-based rendering. Measured cost: uniform
~2.4–2.9× overhead vs raw htmd (see BENCHMARKS.md, M1 section).

## Approach

Port htmd 0.5.4's rendering core (~3.2k lines: `dom_walker.rs`,
`text_util.rs`, `html_escape.rs`, `element_handler/*`) from
`markup5ever_rcdom` to scraper's `ego-tree`, as `crates/htmlmd-core/src/native/`.
Byte-identical output is the contract — htmd's whitespace collapsing and
escaping semantics are ported verbatim, not reinvented.

**Licensing**: adapted files carry
`// SPDX-License-Identifier: Apache-2.0` plus an attribution line
(`Portions adapted from htmd v0.5.4, © letmutex, Apache-2.0`) — they are
derived work and are NOT dual-licensed MIT. The project README notes this.

## Pipeline change

`ConverterBackend` gains a DOM-level entry point with a compatible default:

```rust
pub trait ConverterBackend {
    fn convert(&self, html: &str, options: &ConversionOptions) -> Result<ConversionResult>;
    /// Convert an already-parsed (cleaned) document. Default serializes and
    /// delegates to `convert`, preserving old backends' behavior.
    fn convert_dom(&self, document: &scraper::Html, options: &ConversionOptions) -> Result<ConversionResult> {
        self.convert(&document.html(), options)
    }
}
```

- `cleanup` exposes `clean_html_to_dom(...) -> Result<(Html, ExtractedMetadata)>`;
  the string `clean_html` becomes a thin serialize wrapper over it.
- `lib.rs::convert_with_backend` uses `clean_html_to_dom` + `convert_dom`:
  with `NativeBackend` the document is parsed exactly once.
- `NativeBackend` implements `convert_dom` natively and `convert` as
  parse-then-convert_dom.

## Handler architecture

Mirror htmd's `ElementHandler` model over `ego-tree` so the ~1.1k lines of
custom handlers in `htmd_handlers.rs` port mechanically. The native walker
consumes a ported copy of htmd's `Options` (via the existing
`build_htmd_options` mapping, renamed) so `ConversionOptions` semantics stay
identical. `skip_tags` and `scripting_enabled` behavior ported as-is.

## Phases

- **A** (agent): port core walker + standard handlers; add `convert_dom` +
  `NativeBackend`; differential parity test (`tests/native_parity.rs`):
  every `fixtures/*.html` with `ConversionOptions::default()` must produce
  byte-identical output from `NativeBackend` and `HtmdBackend`. Zero behavior
  change for existing users in this phase.
- **B** (agent): port all custom handlers (semantic inline set, footnotes,
  definition lists, `data-htmlmd-table="html"|"csv"` rendering, math,
  mermaid, alerts, wikilinks, reference links/images with End/Adjacent/
  SectionEnd placement, `htmlmdrule` custom rules, task lists). Extend parity
  to every profile × every fixture, byte-exact.
- **C**: flip `convert()` default to `NativeBackend`; move htmd behind a
  `backend-htmd` feature (stays a dev-dependency for the bench baseline);
  make keep-only prune the live tree instead of re-parsing
  (`cleanup.rs` `apply_keep_only`); make `convert_to_writer` stream;
  re-benchmark (target: ≤ ~1.3× vs raw htmd); docs; ship.

## Invariants

- The full test suite (95 tests incl. 44 fixture tests) must stay green at
  every phase boundary; expected/*.md files are never regenerated.
- No `Regex`/`Selector` compilation in per-node code paths.
- The walker may allocate per-node Strings exactly like htmd does — parity
  first, allocation tuning later (measured, in BENCHMARKS.md).
