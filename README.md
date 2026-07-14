# htmlmd

A fast, configurable, cross-platform HTML-to-Markdown converter written in Rust.

## Project status

**Phase 3 is complete.** The workspace, library API, `htmd`-backed conversion engine, CLI, config loading, fixtures, and the major Phase 3 features are implemented and tested:

- Output profiles: `commonmark`, `gfm`, `extended`, `pandoc`, `obsidian`, `mdx-safe`, `plain-text`
- Extended Markdown: footnotes, definition lists, math, GitHub-style alerts, mermaid diagrams
- Semantic tag handling: `mark`, `del`, `ins`, `sub`, `sup`, `kbd`, etc.
- Advanced tables: GFM, HTML fallback, CSV-like, flatten, drop strategies
- Code-block language detection from classes and source heuristics
- Custom per-selector rules (`drop`, `unwrap`, `text`, `html`, `markdown-template`, `fenced-block`, `link`, `image`)
- Image modes: inline, skip, alt-text, reference
- Reference link placement: end, adjacent, section-end
- DOM/output safety limits

See [`docs/OPTION_REFERENCE.md`](docs/OPTION_REFERENCE.md) for the implementation status of every option.

## Documentation

- [`docs/PROFILES.md`](docs/PROFILES.md) – choosing and using output profiles with examples.
- [`docs/BUILD_AND_DEPLOY.md`](docs/BUILD_AND_DEPLOY.md) – building on Linux, Windows and macOS, plus deployment options.
- [`docs/PACKAGING.md`](docs/PACKAGING.md) – creating winget and apt-get packages.
- [`docs/API_AND_WEB_SERVICE.md`](docs/API_AND_WEB_SERVICE.md) – running the HTTP API server.
- [`docs/OPTION_REFERENCE.md`](docs/OPTION_REFERENCE.md) – every configuration option and CLI flag.

## LLM skill files

This repository includes project-specific instructions for AI coding assistants. Keep these in sync when you change the tool's behavior, flags, or API.

| Tool | Skill location |
|------|----------------|
| Kimi Code CLI | `.kimi/skills/htmlmd/SKILL.md` |
| Claude / Codex | `.claude/skills/htmlmd/SKILL.md` |
| Cursor | `.cursorrules` |
| Generic agents | `AGENTS.md` |

### How to add or update a skill

1. Edit the relevant Markdown file for the tool you are targeting.
2. Mirror the same information to the other skill files so all assistants stay consistent.
3. Keep examples copy-pasteable and based on the files in `fixtures/`.
4. If you add a new CLI flag, API endpoint, or profile, update every skill file and `AGENTS.md`.

To add support for a new assistant, create its standard skill file in this repo (for example, `.copilot/skills/htmlmd/SKILL.md`) and point to it from `AGENTS.md`.

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
```

### Profiles

```bash
# GFM (tables, task lists, strikethrough, autolinks)
htmlmd --profile gfm fixtures/table.html

# Extended (footnotes, definition lists, math, alerts, mermaid)
htmlmd --profile extended fixtures/extended.html

# Obsidian (wikilinks + YAML frontmatter from metadata)
htmlmd --profile obsidian --metadata-title --metadata-description fixtures/extended.html

# Extract metadata into the result (title, description, canonical URL)
htmlmd --metadata-title --metadata-description --metadata-canonical-url fixtures/basic.html

# Pandoc (raw HTML preserved, smart punctuation)
htmlmd --profile pandoc fixtures/extended.html

# MDX-safe (no raw HTML, JSX braces escaped)
htmlmd --profile mdx-safe fixtures/extended.html

# Plain text (readable text, no Markdown markup)
htmlmd --profile plain-text fixtures/basic.html
```

### Batch conversion

```bash
# Output directory with a manifest
htmlmd --output-dir out/ --manifest manifest.json fixtures/*.html

# Mirror a directory tree
htmlmd --recursive --mirror --output-dir out/ docs/

# Check existing outputs without writing
htmlmd --check --diff -o output.md fixtures/basic.html

# Parallel jobs
htmlmd --jobs 4 --output-dir out/ fixtures/*.html
```

### Configuration

```bash
# Print the default config
htmlmd --print-default-config

# Use a config file
htmlmd --config htmlmd.toml fixtures/basic.html

# Override via environment
HTMLMD_PROFILE=gfm HTMLMD_RENDER__HR_STYLE=asterisks htmlmd fixtures/basic.html
```

Example `htmlmd.toml`:

```toml
profile = "extended"

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
image-mode = "inline"

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

### Reference links and images

```bash
# Reference-style links with definitions placed after each link
htmlmd --link-style reference --reference-placement adjacent fixtures/links.html

# Reference-style images
htmlmd --image-mode reference fixtures/image_mode.html

# Drop images entirely, or keep only alt text
htmlmd --image-mode skip fixtures/image_mode.html
htmlmd --image-mode alt-text fixtures/image_mode.html
```

Equivalent config:

```toml
[render]
link-style = "reference"
reference-placement = "adjacent"   # or "end", "section-end"

[cleanup]
image-mode = "reference"
```

### Install

```bash
cargo install --path crates/htmlmd-cli
```

### Test

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

### Benchmark

```bash
cargo bench -p htmlmd-core --bench convert_bench
```

## Library API

```rust
use htmlmd_core::{convert, ConversionOptions};

let result = convert("<h1>Hello</h1>", &ConversionOptions::gfm())?;
println!("{}", result.markdown);
```

Profile-specific constructors:

```rust
ConversionOptions::commonmark();
ConversionOptions::gfm();
ConversionOptions::extended();
ConversionOptions::pandoc();
ConversionOptions::obsidian();
ConversionOptions::mdx_safe();
ConversionOptions::plain_text();
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

Custom backend:

```rust
use htmlmd_core::{ConverterBackend, ConversionOptions, ConversionResult};

struct MyBackend;
impl ConverterBackend for MyBackend {
    fn convert(&self, html: &str, options: &ConversionOptions) -> htmlmd_core::Result<ConversionResult> {
        // Your own conversion engine
        Ok(ConversionResult { markdown: html.to_string(), ..Default::default() })
    }
}
```

## Configuration layers

The effective configuration is built in this order (later overrides earlier):

1. `ConversionOptions::default()`
2. Discovered user config (`$CONFIG_DIR/htmlmd/config.toml`)
3. Discovered project config (`.htmlmd.toml`)
4. Explicit `--config` file
5. Environment variables (`HTMLMD_*`, nested keys separated by `__`)
6. CLI flags

All options are validated before any file is processed.

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

| Profile      | Status         | Notes                                            |
|--------------|----------------|--------------------------------------------------|
| `commonmark` | ✅ implemented | Conservative CommonMark                          |
| `gfm`        | ✅ implemented | Tables, task lists, strikethrough, autolinks     |
| `extended`   | ✅ implemented | GFM + footnotes, definition lists, math, alerts, mermaid |
| `pandoc`     | ✅ implemented | Raw HTML preserved, smart punctuation            |
| `obsidian`   | ✅ implemented | Wikilinks, YAML frontmatter, callouts            |
| `mdx-safe`   | ✅ implemented | Raw HTML stripped/unwrapped, JSX braces escaped  |
| `plain-text` | ✅ implemented | Markdown stripped to readable text               |

## Exit codes

- `0` – success
- `1` – conversion or I/O error
- `2` – CLI or configuration error

## License

Licensed under Apache-2.0.
