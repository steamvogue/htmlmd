# htmlmd

Teaches an agent how to use the `htmlmd` HTML-to-Markdown utility (CLI and HTTP API) in this project.

## What it is

`htmlmd` converts HTML files, strings, or directories into Markdown. It supports multiple output profiles (CommonMark, GFM, Extended, Pandoc, Obsidian, MDX-safe, Plain text), a config file, batch conversion, and a small HTTP API server.

In this repo the compiled binaries live under `dist/<target-triple>/` (on this machine: `dist/aarch64-unknown-linux-gnu/`):

- `dist/aarch64-unknown-linux-gnu/htmlmd` — command-line tool
- `dist/aarch64-unknown-linux-gnu/htmlmd-server` — HTTP API server

If they are missing, build them with:

```bash
cargo build -p htmlmd-cli --release
cargo build -p htmlmd-server --release
```

Or run `scripts/build-release.sh` to build, strip, and copy them into `dist/<target-triple>/` with checksums in one step (kept after `cargo clean`, gitignored).

## When to use

- Convert a single HTML file to Markdown.
- Batch-convert a directory or glob of HTML files.
- Convert HTML from stdin.
- Choose a Markdown dialect/profile for a specific downstream tool (Obsidian, GitHub, MDX, Pandoc).
- Run an HTTP service that converts HTML on demand.

## Basic CLI usage

```bash
# Convert to stdout
dist/aarch64-unknown-linux-gnu/htmlmd fixtures/basic.html

# Write to a file
dist/aarch64-unknown-linux-gnu/htmlmd fixtures/basic.html -o output.md

# Convert from stdin
cat fixtures/basic.html | dist/aarch64-unknown-linux-gnu/htmlmd -

# Use a profile
dist/aarch64-unknown-linux-gnu/htmlmd --profile gfm fixtures/table.html
dist/aarch64-unknown-linux-gnu/htmlmd --profile obsidian --metadata-title fixtures/extended.html
```

## Profiles

| Profile | Use it for | Key behavior |
|---------|------------|--------------|
| `commonmark` | Generic/portable Markdown | Standard CommonMark |
| `gfm` | GitHub/GitLab | Tables, task lists, strikethrough, autolinks |
| `extended` | Static sites, rich notes | Footnotes, definition lists, math `$...$`, alerts, mermaid, `<kbd>`, `<mark>`, etc. |
| `pandoc` | Pandoc workflows | Preserves raw HTML, smart punctuation |
| `obsidian` | Obsidian vaults | `[[wikilinks]]` + YAML frontmatter (with `--metadata-*`) |
| `mdx-safe` | MDX/Docusaurus/Next.js | Escapes JSX braces, drops raw HTML |
| `plain-text` | Search/summaries | Readable text, no Markdown markup, images → alt text |

## Common flags

```bash
-o, --output <path>             # single output file
--output-dir <dir>              # batch root; defaults to current directory
-m, --mirror                    # preserve input-relative paths in batch output
-r, --recursive                 # include descendant .html/.htm files
--profile <name>                # output profile
--metadata-title                # extract <title>
--metadata-description          # extract meta description
--metadata-canonical-url        # extract canonical URL
--link-style inline|reference|collapsed-reference|shortcut-reference
--reference-placement end|adjacent|section-end
--image-mode inline|reference|skip|alt-text
--heading-style atx|setex|keep
--bullet hyphen|asterisk|plus
--code-fence backticks|tildes
--hr-style dashes|asterisks|underscores
--br-style two-spaces|backslash
--skip-tags script,style,nav
--remove-selectors .ad,footer
--unwrap-selectors .content
--keep-only-selectors article
--extract-selector #main
--base-url https://example.com/
--remove-tracking-params
--strict                          # warnings become errors
--config htmlmd.toml
```

## Typical tasks

### Convert a file for GitHub

```bash
dist/aarch64-unknown-linux-gnu/htmlmd --profile gfm fixtures/table.html
```

### Convert to Obsidian with frontmatter

```bash
dist/aarch64-unknown-linux-gnu/htmlmd --profile obsidian \
  --metadata-title --metadata-description \
  fixtures/extended.html
```

### Keep only an article element

```bash
dist/aarch64-unknown-linux-gnu/htmlmd --keep-only-selectors article -o article.md page.html
```

### Reference-style links

```bash
dist/aarch64-unknown-linux-gnu/htmlmd --link-style reference --reference-placement adjacent fixtures/links.html
```

### Drop all images

```bash
dist/aarch64-unknown-linux-gnu/htmlmd --image-mode skip fixtures/image_mode.html
```

### Batch conversion

```bash
dist/aarch64-unknown-linux-gnu/htmlmd fixtures/ --output-dir out/
dist/aarch64-unknown-linux-gnu/htmlmd 'fixtures/*.html' --output-dir out/ --manifest manifest.json
dist/aarch64-unknown-linux-gnu/htmlmd site/ -r -m --output-dir out/
```

### Use a config file

```bash
dist/aarch64-unknown-linux-gnu/htmlmd --config htmlmd.toml fixtures/basic.html
```

Example `htmlmd.toml`:

```toml
profile = "extended"

[render]
link-style = "reference"
reference-placement = "adjacent"

[cleanup]
image-mode = "reference"
remove-tracking-params = true
base-url = "https://example.com/"

[semantic]
footnotes = true
definition-lists = true
```

## HTTP API server

Start the server:

```bash
dist/aarch64-unknown-linux-gnu/htmlmd-server
# listens on http://127.0.0.1:3000
```

Endpoints:

- `GET /health` — health check
- `POST /convert` — convert HTML to Markdown

Example:

```bash
curl -s -X POST http://127.0.0.1:3000/convert \
  -H 'Content-Type: application/json' \
  -d '{
    "html": "<h1>Hello</h1>",
    "options": { "profile": "gfm" }
  }'
```

Response:

```json
{"markdown":"# Hello\n","title":null,"description":null,"canonical_url":null,"diagnostics":[]}
```

For Apache or Nginx reverse proxying, TLS, authentication, and automatic
restart configuration, see `docs/SERVER_DEPLOYMENT.md`.

## Gotchas

- If `dist/aarch64-unknown-linux-gnu/htmlmd` is missing, run `cargo build -p htmlmd-cli --release` and look in `target/release/htmlmd`.
- `--profile obsidian` only emits YAML frontmatter when metadata extraction flags are provided **and** the HTML contains the metadata.
- Wikilinks (`[[...]]`) are only generated in the `obsidian` profile for `<a class="wikilink">` or `<a rel="wikilink">` elements.
- The `target/` directory can be very large; use `cargo clean` when you are done building, but keep the binary you need.

## More docs

- `docs/PROFILES.md` — profile details and examples
- `docs/API_AND_WEB_SERVICE.md` — running the HTTP service
- `docs/SERVER_DEPLOYMENT.md` — production proxy, authentication, and supervision
- `docs/OPTION_REFERENCE.md` — every option and CLI flag
- `docs/BUILD_AND_DEPLOY.md` — building for Linux/Windows/macOS
- `docs/PACKAGING.md` — winget and apt-get packaging
