# htmlmd

A fast, configurable, cross-platform HTML-to-Markdown converter written in
Rust — available as a library (`htmlmd-core`), a CLI (`htmlmd`), and an HTTP
API server (`htmlmd-server`).

## Highlights

- **7 output profiles** — `commonmark`, `gfm`, `extended`, `pandoc`,
  `obsidian`, `mdx-safe`, `plain-text` — so the output matches what your
  renderer actually supports.
- **Extended Markdown** — footnotes, definition lists, math, GitHub-style
  alerts, mermaid diagrams, semantic tags (`mark`, `ins`, `sub`, `kbd`, …).
- **Table strategies** — GFM pipe tables with HTML fallback for complex
  tables, plus CSV-like, flatten, and drop modes.
- **Custom per-selector rules** — `drop`, `unwrap`, `text`, `html`,
  `markdown-template`, `fenced-block`, `link`, `image` actions keyed on CSS
  selectors.
- **Content cleanup** — tag removal, tracking-parameter stripping, base-URL
  resolution, lazy-image/srcset handling, code-language detection.
- **Metadata extraction** — title, description, canonical URL, optionally as
  YAML frontmatter (Obsidian-ready).
- **Batch conversion** — parallel jobs, directory mirroring, JSON manifests,
  `--check --diff` verification for CI.
- **Safety limits** — caps on input size, output size, DOM depth, and
  attribute length for untrusted input.
- **Layered configuration** — defaults → user config → project config →
  `--config` file → `HTMLMD_*` environment variables → CLI flags.

## Install

```bash
git clone https://github.com/steamvogue/htmlmd.git
cd htmlmd
cargo install --path crates/htmlmd-cli
```

Prebuilt per-architecture binaries and packaging (Docker, winget, apt) are
covered in [`docs/BUILD_AND_DEPLOY.md`](docs/BUILD_AND_DEPLOY.md) and
[`docs/PACKAGING.md`](docs/PACKAGING.md).

## Quick start

```bash
# Convert a file to stdout
htmlmd page.html

# From stdin
curl -s https://example.com | htmlmd -

# Pick a profile
htmlmd --profile gfm page.html
htmlmd --profile obsidian --metadata-title --metadata-description page.html

# Write to a file
htmlmd -o page.md page.html
```

### Profiles

| Profile      | Notes                                                     |
|--------------|-----------------------------------------------------------|
| `commonmark` | Conservative CommonMark                                   |
| `gfm`        | Tables, task lists, strikethrough, autolinks              |
| `extended`   | GFM + footnotes, definition lists, math, alerts, mermaid  |
| `pandoc`     | Raw HTML preserved                                        |
| `obsidian`   | Wikilinks, YAML frontmatter, callouts                     |
| `mdx-safe`   | Raw HTML stripped/unwrapped, JSX braces escaped           |
| `plain-text` | Markdown stripped to readable text                        |

See [`docs/PROFILES.md`](docs/PROFILES.md) for details and examples.

### Batch conversion

```bash
# Output directory with a manifest
htmlmd --output-dir out/ --manifest manifest.json pages/*.html

# Mirror a directory tree
htmlmd --recursive --mirror --output-dir out/ site/

# Verify existing outputs without writing (CI-friendly)
htmlmd --check --diff -o page.md page.html

# Parallel jobs
htmlmd --jobs 4 --output-dir out/ pages/*.html
```

### Configuration

```bash
htmlmd --print-default-config          # dump the default config
htmlmd --config htmlmd.toml page.html  # use a config file
HTMLMD_PROFILE=gfm htmlmd page.html    # override via environment
```

Example `htmlmd.toml`:

```toml
profile = "extended"

[render]
heading-style = "atx"
bullet = "hyphen"
link-style = "reference"
reference-placement = "adjacent"   # or "end", "section-end"

[cleanup]
remove-tags = ["script", "style", "nav"]
remove-tracking-params = true
base-url = "https://example.com/"
image-mode = "inline"              # or "skip", "alt-text", "reference"

[semantic]
footnotes = true
definition-lists = true
detect-languages = true
mermaid = "fenced"

[[extension.custom-rules]]
selectors = [".ad"]
action = "drop"
priority = 10

[[extension.custom-rules]]
selectors = ["span.badge"]
action = "markdown-template"
template = "**{text}**"
priority = 0

[limits]
max-input-bytes = 50_000_000
max-output-bytes = 10_000_000
max-dom-depth = 256
max-attribute-len = 10_000
```

Configuration is layered (later overrides earlier): built-in defaults →
`$CONFIG_DIR/htmlmd/config.toml` → project `.htmlmd.toml` → `--config` file →
`HTMLMD_*` environment variables (nested keys via `__`) → CLI flags. All
options are validated before any file is processed. Every option and flag is
documented in [`docs/OPTION_REFERENCE.md`](docs/OPTION_REFERENCE.md).

## Library

```rust
use htmlmd_core::{convert, ConversionOptions};

let result = convert("<h1>Hello</h1>", &ConversionOptions::gfm())?;
println!("{}", result.markdown);
```

Profile constructors (`ConversionOptions::gfm()`, `::obsidian()`, …), a
streaming `convert_to_writer` API, and a pluggable `ConverterBackend` trait
are available — see the crate docs in `crates/htmlmd-core`.

## HTTP API server

```bash
cargo run -p htmlmd-server --release
# POST /convert with HTML body → Markdown
```

See [`docs/API_AND_WEB_SERVICE.md`](docs/API_AND_WEB_SERVICE.md).

## Exit codes

- `0` – success
- `1` – conversion or I/O error
- `2` – CLI or configuration error

## Documentation

- [`docs/PROFILES.md`](docs/PROFILES.md) – choosing and using output profiles.
- [`docs/OPTION_REFERENCE.md`](docs/OPTION_REFERENCE.md) – every option and CLI flag.
- [`docs/BUILD_AND_DEPLOY.md`](docs/BUILD_AND_DEPLOY.md) – building per platform, deployment.
- [`docs/PACKAGING.md`](docs/PACKAGING.md) – winget and apt packages.
- [`docs/API_AND_WEB_SERVICE.md`](docs/API_AND_WEB_SERVICE.md) – the HTTP API server.
- [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) – project status, workspace layout, contributor/AI-assistant notes.
- [`docs/ROADMAP.md`](docs/ROADMAP.md) – performance & quality roadmap.

## License

Licensed under MIT OR Apache-2.0, with one exception: the files under
`crates/htmlmd-core/src/native/` that are adapted from the
[htmd](https://github.com/letmutex/htmd) crate are Apache-2.0 only (see their
SPDX headers and attribution lines).
