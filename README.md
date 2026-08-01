# htmlmd

[![CI](https://github.com/steamvogue/htmlmd/actions/workflows/ci.yml/badge.svg)](https://github.com/steamvogue/htmlmd/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![crates.io](https://img.shields.io/crates/v/htmlmd-core.svg)](https://crates.io/crates/htmlmd-core)

**Turn noisy HTML into clean, model-ready Markdown or text for AI agents, RAG
pipelines, and LLM applications.**

`htmlmd` is a fast Rust data-preparation toolkit available as a library
(`htmlmd-core`), CLI (`htmlmd`), and HTTP API server (`htmlmd-server`). It
removes page chrome, normalizes content, extracts metadata, and produces
predictable Markdown or plain text before web content reaches your model.

![htmlmd turns noisy webpage HTML into clean Markdown for AI](docs/assets/htmlmd-ai-before-after.svg)

## Built for AI data preparation

- **Agent tools** — give browsing and research agents focused page content
  instead of navigation, ads, scripts, and tracking parameters.
- **RAG ingestion** — batch-convert captured HTML into consistent Markdown,
  readable text, and metadata for chunking and indexing.
- **LLM context pipelines** — select the useful DOM, normalize whitespace and
  links, and choose a stable output profile before prompting a model.
- **Production services** — embed the Rust library, call the CLI, or run the
  guarded HTTP API with input, output, and DOM-depth limits.

```bash
# Fetch with your crawler or HTTP client, then prepare the article for AI.
curl -s https://example.com/article \
  | htmlmd --profile gfm --keep-only-selectors article \
      --remove-tracking-params true -
```

`htmlmd` converts HTML you provide; pair it with your crawler, browser, or HTTP
client when you also need page fetching or JavaScript rendering.

## What's new in 0.1.2

Input selection now uses one consistent syntax: positional inputs accept files,
directories, and glob masks. A directory selects direct `.html`/`.htm` files;
add `-r` / `--recursive` for descendants. Directory, glob, and multi-file
inputs are treated as batches and default to the current directory when
`--output-dir` is omitted. `-m` / `--mirror` preserves input-relative paths,
and flat batches now reject output-name collisions before writing. See the
[changelog](CHANGELOG.md) for details.

## Highlights

- **7 output profiles** — `commonmark`, `gfm`, `extended`, `pandoc`,
  `obsidian`, `mdx-safe`, `plain-text` — so the output matches what your
  renderer actually supports.
- **Extended Markdown** — footnotes, definition lists, math, GitHub-style
  alerts, mermaid diagrams, semantic tags (`mark`, `ins`, `sub`, `kbd`, …).
- **Table strategies** — GFM pipe tables with flatten fallback for complex
  tables (rowspan/colspan, irregular columns), plus CSV-like, HTML fallback,
  and drop modes.
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

## Performance

Full CLI runs over identical input on a Raspberry Pi 5 (aarch64), GFM output,
lower is better:

| tool | 1 MB article | 245 KB API docs |
|---|---|---|
| **htmlmd** | **59 ms** | **30 ms** |
| html-to-markdown v2 (Go) | 174 ms | 23 ms |
| turndown (Node) | 1482 ms | 412 ms |
| markdownify (Python) | 1164 ms | 286 ms |
| pandoc | 5761 ms | 883 ms |

Go's html-to-markdown v2 wins the code-heavy column — htmlmd runs
code-language detection there that v2 does not implement. Everything else,
including pandoc (the only other multi-flavor converter), is 10–98× slower.
The whole htmlmd pipeline — cleanup, tracking-param stripping, language
detection, metadata, safety limits — costs 8–19% over the bare `htmd`
library it renders with.

Two honest caveats: as a *library*,
[`fast_html2md`](https://github.com/spider-rs/html2md) is 1.5–2.4× faster
than htmlmd because it streams instead of building a DOM — if you want plain
single-flavor conversion and nothing else, use it. htmlmd builds a tree
because profiles, selector rules, and metadata need one. And parsing is
quadratic in nesting depth (an `html5ever` trait shared by most Rust HTML
tools), so bound `max-input-bytes` for untrusted input.

Method, full results, and the reproducible harness:
[`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) · [`benches/compare/`](benches/compare/).

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

## Use examples

### One document

```bash
# Print Markdown to stdout
htmlmd page.html

# Write one explicit output file
htmlmd page.html -o page.md

# Convert HTML received on stdin
curl -s https://example.com/article | htmlmd -
```

One explicit file writes to stdout unless `-o` is provided. A directory, glob,
or multiple input paths signals a batch and writes `.md` files to the current
directory by default. Use `--output-dir DIR` to choose another batch root.

### Directory batches and mirroring

Given this input tree:

```text
site/
├── index.html
├── about.htm
├── ignored.txt
└── docs/
    └── guide.html
```

the output mapping is:

| Command | Selected inputs | Outputs |
|---|---|---|
| `htmlmd site/` | Direct `.html`/`.htm` files | `./index.md`, `./about.md` |
| `htmlmd site/ -r` | Direct files and descendants | `./index.md`, `./about.md`, `./guide.md` |
| `htmlmd site/ -r -m` | Direct files and descendants | `./index.md`, `./about.md`, `./docs/guide.md` |
| `htmlmd site/ -r -m --output-dir out/` | Direct files and descendants | `out/index.md`, `out/about.md`, `out/docs/guide.md` |

Without `-m`, batch outputs are flattened into the output root. With `-m`,
each output keeps the input path relative to the selected directory (or to a
glob's non-wildcard prefix). `--mirror` changes output mapping only; it does not
select additional input files. If a flat batch contains duplicate basenames,
`htmlmd` stops before writing and recommends `-m`.

Batch output uses the `overwrite` policy by default. Use an explicit
`--output-dir out/` to keep generated files isolated, or add
`--output-policy fail-if-exists` when existing files must never be replaced.

### Masks and multiple input paths

```bash
# Quote a mask when htmlmd should expand it rather than the shell
htmlmd 'pages/*.html'

# Select more than one source; outputs default to the current directory
htmlmd intro.html reference.htm

# Put a batch elsewhere and record a manifest
htmlmd 'pages/*.html' --output-dir out/ --manifest manifest.json

# Run conversions with four workers
htmlmd site/ -r -m --output-dir out/ --jobs 4
```

### Verification and AI preparation

```bash
# Verify an existing generated file without changing it
htmlmd page.html -o page.md --check --diff

# Keep the article, strip tracking parameters, and emit model-ready GFM
curl -s https://example.com/article \
  | htmlmd - --profile gfm --keep-only-selectors article \
      --remove-tracking-params true
```

## Profiles

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

## Configuration

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

See [`docs/API_AND_WEB_SERVICE.md`](docs/API_AND_WEB_SERVICE.md) for the API and
[`docs/SERVER_DEPLOYMENT.md`](docs/SERVER_DEPLOYMENT.md) for private-backend
Apache/Nginx proxying, authentication, and automatic restart configuration.

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
- [`docs/SERVER_DEPLOYMENT.md`](docs/SERVER_DEPLOYMENT.md) – production reverse proxies, authentication, and process supervision.
- [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) – method, results, and the reproducible harness.
- [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) – project status, workspace layout, contributor/AI-assistant notes.
- [`docs/ROADMAP.md`](docs/ROADMAP.md) – performance & quality roadmap.
- [`docs/RELEASING.md`](docs/RELEASING.md) – release runbook.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) · [`CHANGELOG.md`](CHANGELOG.md)

## License

Licensed under MIT OR Apache-2.0, with one exception: the files under
`crates/htmlmd-core/src/native/` that are adapted from the
[htmd](https://github.com/letmutex/htmd) crate are Apache-2.0 only (see their
SPDX headers and attribution lines).
