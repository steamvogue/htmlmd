# Contributing to htmlmd

Thanks for your interest. This file covers what you need to get a change
merged; [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) covers the workspace
layout and [`docs/ROADMAP.md`](docs/ROADMAP.md) covers where the project is
going.

## Getting started

```bash
git clone https://github.com/steamvogue/htmlmd.git
cd htmlmd
cargo test --workspace
```

Rust 1.85+ (the workspace `rust-version`). No other toolchain needed.

## Before opening a pull request

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

CI runs exactly these, plus `cargo-deny`, on Linux, macOS, and Windows.

## House rules

**Never regenerate `fixtures/expected/*.md` to make a test pass.** Those files
are the conversion contract. If your change alters one, that is a finding:
inspect the diff, convince yourself the new output is more correct, and say so
explicitly in the PR. Nearly every change should leave them untouched.

**The two backends must agree byte-for-byte.** `tests/native_parity.rs` asserts
that the native renderer and the legacy `htmd` backend produce identical output
across every profile and fixture. If you touch rendering, keep it green; if you
touch cleanup, both backends see the change and parity holds automatically.

**No performance claims without numbers.** Add or update a row in
[`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) with before/after measurements from
the same machine. `cargo bench -p htmlmd-core --bench convert_bench`. There is
also an ignored per-pass profiler for the cleanup pipeline:

```bash
cargo test -p htmlmd-core --release pass_timing -- --ignored --nocapture
```

**Don't add options that don't do anything.** A config field that parses but
never affects output is worse than no field: it promises a capability that
isn't there. Twenty of them were removed for exactly this reason. Wire it, or
leave it out.

**Hot paths compile nothing per node.** Regexes and CSS selectors are compiled
once per process (`once_cell::sync::Lazy`, `regex_cache.rs`) or once per
conversion — never inside a per-element loop. That mistake previously cost a
50× slowdown.

## Adding a feature

1. Add an HTML fixture under `fixtures/` and its expected Markdown under
   `fixtures/expected/`, or a focused test in `crates/htmlmd-core/tests/`.
2. Implement it. DOM-level work belongs in `cleanup.rs`; Markdown rendering
   belongs in `native/`. If the feature needs a selector, match it in the DOM
   pass and mark the element for the renderer — see how custom rules do it.
3. Document the option in [`docs/OPTION_REFERENCE.md`](docs/OPTION_REFERENCE.md)
   with its real implementation status.
4. Update the LLM skill files if you changed flags, profiles, or the API — see
   [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md).

## Reporting bugs

A failing input beats a description. Please include the HTML (minimized if you
can), the options or CLI flags used, what you expected, and what you got.
Security-relevant reports (crashes, resource exhaustion, anything reachable
through the HTTP server) are especially welcome.

## Licensing

Contributions are licensed under `MIT OR Apache-2.0`, matching the project.
The files under `crates/htmlmd-core/src/native/` adapted from
[htmd](https://github.com/letmutex/htmd) are Apache-2.0 only; keep their SPDX
headers and attribution intact when editing them.
