# htmlmd option reference

This document lists the configuration options accepted by `htmlmd-core` and the CLI. The schema is stable; the effect column indicates what is implemented and tested through **Phase 2**.

Legend:
- ✅ Implemented and tested
- ⚠️ Parsed/accepted but behavior is limited or fallback
- ❌ Reserved for a later phase

## Top-level

| Option    | Type   | Default      | Phase 2 | Description                              |
|-----------|--------|--------------|---------|------------------------------------------|
| `profile` | string | `commonmark` | ✅      | Output profile (`commonmark`, `gfm`, …)  |
| `strict`  | bool   | `false`      | ✅      | Turn warnings into errors                |

## `[render]` – Markdown rendering

| Option                    | Type   | Default       | Status | Notes                                        |
|---------------------------|--------|---------------|--------|----------------------------------------------|
| `heading-style`           | enum   | `atx`         | ✅     | `atx`, `setex`, `keep` maps to `atx`         |
| `bullet`                  | enum   | `hyphen`      | ✅     | `hyphen`/`plus` map to dash; `asterisk` works|
| `ordered-list-marker`     | enum   | `decimal`     | ⚠️     | Schema only; `htmd` always uses decimals     |
| `emphasis-marker`         | enum   | `asterisk`    | ⚠️     | Schema only                                  |
| `strong-marker`           | enum   | `asterisk`    | ⚠️     | Schema only                                  |
| `code-fence`              | enum   | `backticks`   | ✅     | `backticks` or `tildes`                      |
| `code-fence-min-length`   | u8     | `3`           | ⚠️     | Schema only                                  |
| `line-wrapping`           | enum   | `off`         | ❌     | Reserved                                     |
| `hard-break-style`        | enum   | `two-spaces`  | ✅     | `two-spaces` or `backslash`                  |
| `hr-style`                | enum   | `dashes`      | ✅     | `dashes`, `asterisks`, `underscores`         |
| `escaping-mode`           | enum   | `minimal`     | ⚠️     | Schema only; `htmd` escaping is used         |
| `character-entities`      | enum   | `decode`      | ⚠️     | Schema only                                  |
| `unicode-normalization`   | enum   | `off`         | ✅     | `off`, `nfc`, `nfkc` post-processing         |
| `smart-punctuation`       | enum   | `preserve`    | ⚠️     | Schema only                                  |
| `trailing-whitespace`     | enum   | `trim`        | ✅     | `trim` or `preserve`                         |
| `final-newline`           | enum   | `ensure`      | ✅     | `ensure`, `preserve`, `suppress`             |
| `blank-line-compaction`   | u8     | `1`           | ⚠️     | Schema only                                  |
| `link-style`              | enum   | `inline`      | ✅     | `inline`, `reference`, `collapsed-reference`, `shortcut-reference` |
| `reference-placement`     | enum   | `end`         | ⚠️     | Reference links always placed at end         |
| `image-mode`              | enum   | `inline`      | ✅     | `inline`, `skip`, `alt-text` wired; `reference` reserved |
| `title-attribute`         | enum   | `ignore`      | ✅     | `ignore` strips titles; others not wired     |
| `url-escaping`            | enum   | `auto`        | ⚠️     | Schema only                                  |
| `autolink-detection`      | bool   | `true`        | ⚠️     | Schema only                                  |
| `email-handling`          | enum   | `mailto`      | ⚠️     | Schema only                                  |
| `raw-html-policy`         | enum   | `drop`        | ✅     | `faithful` enables `htmd` faithful mode      |
| `comment-policy`          | enum   | `drop`        | ⚠️     | Schema only                                  |
| `doctype-policy`          | enum   | `drop`        | ⚠️     | Schema only                                  |

## `[cleanup]` – HTML cleanup and content selection

| Option                    | Type     | Default                         | Status | Notes |
|---------------------------|----------|----------------------------------|--------|-------|
| `remove-selectors`        | [string] | `[]`                             | ✅     | CSS selectors whose matches are dropped; validated at load time |
| `unwrap-selectors`        | [string] | `[]`                             | ✅     | CSS selectors whose matches are unwrapped |
| `keep-only-selectors`     | [string] | `[]`                             | ✅     | Keep only first matching selector |
| `extract-selector`        | string?  | `null`                           | ✅     | Alias for keep-only |
| `main-content-selector`   | string?  | `null`                           | ✅     | Alias for keep-only |
| `remove-tags`             | [string] | `head`, `script`, `style`, …     | ✅     | Tag names dropped before conversion |
| `per-tag-behavior`        | map      | `{}`                             | ✅     | `drop`, `unwrap`, `text`, `html` per tag |
| `hidden-content-policy`   | enum     | `hide`                           | ✅     | Removes `display:none`, `hidden`, `aria-hidden` |
| `metadata`                | object   | all `false`                      | ✅     | Extract title/description/canonical/OG/Twitter |
| `base-url`                | string?  | `null`                           | ✅     | Resolve relative `href`/`src`; validated |
| `url-rewrite-rules`       | [object] | `[]`                             | ✅     | Regex-based URL rewriting; patterns validated |
| `remove-tracking-params`  | bool     | `true`                           | ✅     | Strips `utm_*`, `fbclid`, `gclid` |
| `tracking-params`         | [string] | `[]`                             | ✅     | Additional params to strip |
| `allowed-url-schemes`     | [string] | `[]`                             | ✅     | Allow-list override |
| `blocked-url-schemes`     | [string] | `javascript`, `data`, `file`     | ✅     | Block dangerous schemes |
| `lazy-image-attrs`        | [string] | `data-src`, `data-original`      | ✅     | Fill missing `src` |
| `responsive-image-policy` | enum     | `first-srcset`                   | ✅     | `first-srcset`, `largest`, `preserve-srcset` |
| `preserve-image-metadata` | bool     | `false`                          | ✅     | Append `widthxheight` to `alt` |
| `image-mode`              | enum     | `inline`                         | ✅     | See `[render].image-mode` mirror |
| `media-policy`            | enum     | `inline`                         | ✅     | `drop`, `placeholder` wired; `inline`/`link` partial |
| `form-handling`           | enum     | `drop`                           | ✅     | `drop`, `readable` wired; `checklist` reserved |
| `details-handling`        | enum     | `expand`                         | ✅     | `expand`, `summary-only`, `drop` |
| `custom-element-policy`   | enum     | `unwrap`                         | ✅     | `unwrap`, `drop`, `preserve-html` for hyphenated tags |

## `[semantic]` – Semantic conversion rules

| Option                    | Type     | Default                          | Status | Notes |
|---------------------------|----------|----------------------------------|--------|-------|
| `heading-offset`          | i8       | `0`                              | ⚠️     | Schema only |
| `normalize-headings`      | bool     | `false`                          | ⚠️     | Schema only |
| `list-indent`             | u8       | `4`                              | ⚠️     | Schema only |
| `task-lists`              | bool     | `true`                           | ⚠️     | Schema only; `htmd` handles some checkboxes |
| `table-handling`          | enum     | `gfm`                            | ✅     | GFM pipe tables via `htmd` |
| `difficult-table-strategy`| enum     | `html-fallback`                  | ⚠️     | Schema only |
| `code-language-patterns`  | [string] | `language-*`, `lang-*`, …        | ⚠️     | Parsed/validated; `htmd` extracts `class` language |
| `inline-style-subset`     | enum     | `basic`                          | ⚠️     | Schema only |
| `semantic-tags`           | enum     | `convert`                        | ⚠️     | Schema only |
| `definition-lists`        | bool     | `false`                          | ❌     | Reserved |
| `footnotes`               | bool     | `false`                          | ❌     | Reserved |
| `math`                    | object   | `enabled: false`                 | ❌     | Reserved |
| `mermaid`                 | enum     | `fenced`                         | ❌     | Reserved |
| `embedded-media`          | enum     | `preserve-link`                  | ❌     | Reserved |

## `[extension]` – Extensibility

| Option        | Type     | Default | Status | Notes |
|---------------|----------|---------|--------|-------|
| `custom-rules`| [object] | `[]`    | ❌     | Reserved |
| `rule-packs`  | [string] | `[]`    | ❌     | Reserved |

## `[limits]` – Safety and size limits

| Option              | Type | Default | Status | Notes |
|---------------------|------|---------|--------|-------|
| `max-input-bytes`   | u64  | `0`     | ✅     | `0` = unlimited |
| `max-output-bytes`  | u64  | `0`     | ⚠️     | Parsed, not enforced |
| `max-dom-depth`     | u32  | `0`     | ⚠️     | Parsed, not enforced |
| `max-node-count`    | u64  | `0`     | ✅     | `0` = unlimited |
| `max-attribute-len` | u64  | `0`     | ⚠️     | Parsed, not enforced |

## CLI-only flags

| Flag                      | Status | Notes |
|---------------------------|--------|-------|
| `--output-dir`            | ✅     | Batch output directory |
| `--mirror`                | ✅     | Preserve directory tree under `--output-dir` |
| `--recursive`             | ✅     | Recurse directories for `.html`/`.htm` |
| `--output-policy`         | ✅     | `overwrite`, `skip-existing`, `fail-if-exists` |
| `--atomic`                | ✅     | Write via temp file + rename |
| `--preserve-timestamps`   | ✅     | Copy input timestamps to output |
| `--manifest`              | ✅     | JSON manifest with hashes and metadata |
| `--check`                 | ✅     | Exit non-zero if output would change |
| `--diff`                  | ✅     | Print line diff with `--check` |
| `--encoding`              | ✅     | Explicit input encoding (BOM auto-detected) |
| `--config`                | ✅     | Explicit TOML/JSON config file |
| `--print-default-config`  | ✅     | TOML dump of defaults |
| `--print-effective-config`| ✅     | JSON dump after merges |
| `--dry-run`               | ✅     | Simulate, no writes |
| `--jobs`                  | ✅     | Rayon thread-pool size |
| `--quiet` / `--verbose`   | ✅     | Output verbosity |

## Configuration layers

The effective configuration is built in this order (later overrides earlier):

1. `ConversionOptions::default()`
2. Discovered user config (`$CONFIG_DIR/htmlmd/config.toml`)
3. Discovered project config (`.htmlmd.toml`)
4. Explicit `--config` file
5. Environment variables (`HTMLMD_*`, nested keys separated by `__`)
6. CLI flags

All options are validated before any file is processed.
