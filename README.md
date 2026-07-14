# htmlmd

A fast, configurable, cross-platform HTML-to-Markdown converter written in Rust.

## Project status

**Phase 1 is complete.** The workspace, public library API, `htmd`-backed conversion engine, CLI, CommonMark/GFM output, config loading, and a fixture/snapshot test suite are implemented and tested.

Advanced features listed in the architecture document (extended profiles, math, custom rules, WASM/Node bindings, local HTTP daemon, full profile implementations, etc.) are scaffolded in the configuration schema but are **not yet wired or tested**. They will be implemented in later phases. See [`docs/OPTION_REFERENCE.md`](docs/OPTION_REFERENCE.md) for the current status of each option group.

## Quick start

### Build

```bash
cargo build --workspace --release
```

### Run the CLI

```bash
# Convert a file to stdout
cargo run -p htmlmd-cli -- fixtures/basic.html

# Or after installation
htmlmd fixtures/basic.html

# Stdin
cat fixtures/basic.html | htmlmd -

# GFM profile, custom output file
htmlmd --profile gfm -o output.md fixtures/table.html

# Batch conversion with manifest
htmlmd --output-dir out/ --manifest manifest.json fixtures/*.html

# Mirror a directory tree
htmlmd --recursive --mirror --output-dir out/ docs/

# Check existing outputs without writing
htmlmd --check --diff -o output.md fixtures/basic.html

# Explicit input encoding
htmlmd --encoding windows-1252 old-page.html

# Use a config file
htmlmd --config htmlmd.toml fixtures/basic.html
```

### Install

```bash
cargo install --path crates/htmlmd-cli
```

### Test

```bash
cargo test --workspace
```

### Benchmark

```bash
cargo bench -p htmlmd-core --bench convert_bench
```

## Library API

```rust
use htmlmd_core::{convert, ConversionOptions};

let md = convert("<h1>Hello</h1>", &ConversionOptions::default())?;
println!("{}", md.markdown);
```

Streaming API:

```rust
use htmlmd_core::{convert_to_writer, ConversionOptions};
use std::io;

convert_to_writer(
    "<p>hello</p>",
    &ConversionOptions::default(),
    &mut io::stdout(),
)?;
```

## Configuration

`htmlmd` accepts options through:

- Rust API (`ConversionOptions`)
- CLI flags (subset; see `--help`)
- TOML or JSON config file (`--config`)
- Environment variables prefixed with `HTMLMD_` and separated by `__` (e.g. `HTMLMD_PROFILE=gfm`, `HTMLMD_RENDER__HR_STYLE=underscores`)

Print the default config:

```bash
htmlmd --print-default-config
```

Example `htmlmd.toml`:

```toml
profile = "gfm"

[render]
heading-style = "atx"
bullet = "hyphen"
link-style = "inline"
code-fence = "backticks"
hr-style = "dashes"
br-style = "two-spaces"

[cleanup]
remove-tags = ["script", "style", "nav"]
remove-tracking-params = true
base-url = "https://example.com/"

[limits]
max-input-bytes = 50_000_000
```

## Workspace layout

```
.
├── Cargo.toml
├── crates/
│   ├── htmlmd-core/      # Reusable library
│   └── htmlmd-cli/       # `htmlmd` binary
├── fixtures/             # HTML fixtures and expected Markdown
├── benches/              # Criterion benchmarks
└── docs/
```

## Profiles

| Profile     | Phase 1 status | Notes                                            |
|-------------|----------------|--------------------------------------------------|
| `commonmark`| ✅ implemented | Conservative CommonMark                          |
| `gfm`       | ✅ implemented | Tables, task lists, strikethrough, autolinks     |
| `extended`  | ⚠️ skeleton    | Accepts config, falls back to GFM behavior       |
| `pandoc`    | ⚠️ skeleton    | Reserved                                         |
| `obsidian`  | ⚠️ skeleton    | Reserved                                         |
| `mdx-safe`  | ⚠️ skeleton    | Reserved                                         |
| `plain-text`| ⚠️ skeleton    | Reserved                                         |

## Exit codes

- `0` – success
- `1` – conversion or I/O error
- `2` – CLI or configuration error

## License

Licensed under either of MIT or Apache-2.0 at your option.
