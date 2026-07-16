# htmlmd option reference

This document lists the configuration options accepted by `htmlmd-core` and the CLI. Every option in the schema is functional; formerly reserved, accepted-but-inert options were removed pre-1.0 (see [Removed options](#removed-options)).

Legend:
- ✅ Implemented and tested
- ⚠️ Parsed/accepted but behavior is limited or fallback

## Top-level

| Option    | Type   | Default      | Status  | Description                              |
|-----------|--------|--------------|---------|------------------------------------------|
| `profile` | string | `commonmark` | ✅      | Output profile (`commonmark`, `gfm`, `extended`, `pandoc`, `obsidian`, `mdx-safe`, `plain-text`)  |
| `strict`  | bool   | `false`      | ✅      | Turn warnings into errors                |

## `[render]` – Markdown rendering

| Option                    | Type   | Default       | Status | Notes                                        |
|---------------------------|--------|---------------|--------|----------------------------------------------|
| `heading-style`           | enum   | `atx`         | ✅     | `atx`, `setex`                               |
| `bullet`                  | enum   | `hyphen`      | ✅     | `hyphen`/`plus` map to dash; `asterisk` works|
| `code-fence`              | enum   | `backticks`   | ✅     | `backticks` or `tildes`                      |
| `hard-break-style`        | enum   | `two-spaces`  | ✅     | `two-spaces` or `backslash`                  |
| `hr-style`                | enum   | `dashes`      | ✅     | `dashes`, `asterisks`, `underscores`         |
| `unicode-normalization`   | enum   | `off`         | ✅     | `off`, `nfc`, `nfkc` post-processing         |
| `trailing-whitespace`     | enum   | `trim`        | ✅     | `trim` or `preserve`                         |
| `final-newline`           | enum   | `ensure`      | ✅     | `ensure`, `preserve`, `suppress`             |
| `link-style`              | enum   | `inline`      | ✅     | `inline`, `reference`, `collapsed-reference`, `shortcut-reference` |
| `reference-placement`     | enum   | `end`         | ✅     | `end`, `adjacent`, `section-end`             |
| `image-mode`              | enum   | `inline`      | ✅     | `inline`, `skip`, `alt-text`, `reference`    |
| `title-attribute`         | enum   | `ignore`      | ⚠️     | `ignore` strips titles; `inline`/`reference` not wired |
| `raw-html-policy`         | enum   | `drop`        | ✅     | `faithful` enables `htmd` faithful mode      |

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
| `form-handling`           | enum     | `drop`                           | ✅     | `drop`, `readable`                             |
| `details-handling`        | enum     | `expand`                         | ✅     | `expand`, `summary-only`, `drop` |
| `custom-element-policy`   | enum     | `unwrap`                         | ✅     | `unwrap`, `drop`, `preserve-html` for hyphenated tags |

## `[semantic]` – Semantic conversion rules

| Option                    | Type     | Default                          | Status | Notes |
|---------------------------|----------|----------------------------------|--------|-------|
| `heading-offset`          | i8       | `0`                              | ✅     | Shift heading levels by a signed amount; result clamped to `h1`..`h6` |
| `task-lists`              | bool     | `true`                           | ✅     | `<input type="checkbox">` list items become `[ ]`/`[x]` task markers |
| `table-handling`          | enum     | `gfm`                            | ✅     | `gfm`, `html-fallback`, `csv-like`, `flatten`, `drop` |
| `difficult-table-strategy`| enum     | `html-fallback`                  | ⚠️     | `html-fallback`/`flatten` wired; `span-cells` reserved |
| `code-language-patterns`  | [string] | `language-*`, `lang-*`, …        | ✅     | Class/language extraction |
| `detect-languages`        | bool     | `true`                           | ✅     | Heuristic language detection for bare code blocks |
| `definition-lists`        | bool     | `false`                          | ✅     | Pandoc-style definition lists |
| `footnotes`               | bool     | `false`                          | ✅     | `[^n]` refs and `[^n]: ...` defs |
| `math`                    | object   | `enabled: false`                 | ✅     | `output`: `inline-dollar` (`$…$`, `$$…$$` for display), `block-dollar` (`$$…$$` always), `fenced`, `plain`, `preserve-html` |
| `mermaid`                 | enum     | `fenced`                         | ✅     | `fenced`, `preserve-html`, `drop` |

## `[extension]` – Extensibility

| Option        | Type     | Default | Status | Notes |
|---------------|----------|---------|--------|-------|
| `custom-rules`| [object] | `[]`    | ✅     | Per-selector actions; see README examples |

## `[limits]` – Safety and size limits

| Option              | Type | Default | Status | Notes |
|---------------------|------|---------|--------|-------|
| `max-input-bytes`   | u64  | `0`     | ✅     | `0` = unlimited |
| `max-output-bytes`  | u64  | `0`     | ✅     | `0` = unlimited |
| `max-dom-depth`     | u32  | `256`   | ✅     | `0` = unlimited. Enforced, not advisory: content nested deeper is pruned with a warning (rejected outright under `strict`). The renderer recurses per DOM level, so removing the default invites a stack overflow on pathological input. |
| `max-node-count`    | u64  | `0`     | ✅     | `0` = unlimited |
| `max-attribute-len` | u64  | `0`     | ✅     | `0` = unlimited |

## CLI-only flags

| Flag                          | Status | Notes |
|-------------------------------|--------|-------|
| `-o`, `--output`              | ✅     | Output file (`-` for stdout) |
| `--output-dir`                | ✅     | Batch output directory |
| `--mirror`                    | ✅     | Preserve directory tree under `--output-dir` |
| `--recursive`                 | ✅     | Recurse directories for `.html`/`.htm` |
| `--output-policy`             | ✅     | `overwrite`, `skip-existing`, `fail-if-exists` |
| `--atomic`                    | ✅     | Write via temp file + rename |
| `--preserve-timestamps`       | ✅     | Copy input timestamps to output |
| `--manifest`                  | ✅     | JSON manifest with hashes and metadata |
| `--check`                     | ✅     | Exit non-zero if output would change |
| `--diff`                      | ✅     | Print line diff with `--check` |
| `--encoding`                  | ✅     | Explicit input encoding (BOM auto-detected) |
| `-c`, `--config`              | ✅     | Explicit TOML/JSON config file |
| `--print-default-config`      | ✅     | TOML dump of defaults |
| `--print-effective-config`    | ✅     | JSON dump after merges |
| `--dry-run`                   | ✅     | Simulate, no writes |
| `--jobs`                      | ✅     | Rayon thread-pool size |
| `--quiet` / `--verbose`       | ✅     | Output verbosity |
| `--profile`                   | ✅     | Output profile shorthand |
| `--heading-style`             | ✅     | `atx`, `setex` |
| `--bullet`                    | ✅     | `hyphen`, `asterisk`, `plus` |
| `--link-style`                | ✅     | `inline`, `reference`, `collapsed-reference`, `shortcut-reference` |
| `--reference-placement`       | ✅     | `end`, `adjacent`, `section-end` |
| `--image-mode`                | ✅     | `inline`, `reference`, `skip`, `alt-text` |
| `--code-fence`                | ✅     | `backticks`, `tildes` |
| `--hr-style`                  | ✅     | `dashes`, `asterisks`, `underscores` |
| `--br-style`                  | ✅     | `two-spaces`, `backslash` |
| `--skip-tags`                 | ✅     | Comma-separated tag names to drop |
| `--remove-selectors`          | ✅     | Comma-separated CSS selectors to drop |
| `--unwrap-selectors`          | ✅     | Comma-separated CSS selectors to unwrap |
| `--keep-only-selectors`       | ✅     | Comma-separated CSS selectors to keep |
| `--extract-selector`          | ✅     | Keep only the first match |
| `--base-url`                  | ✅     | Resolve relative URLs |
| `--remove-tracking-params`    | ✅     | Strip tracking query params |
| `--metadata-title`            | ✅     | Extract `<title>` into result metadata |
| `--metadata-description`      | ✅     | Extract `meta[name="description"]` |
| `--metadata-canonical-url`    | ✅     | Extract `link[rel="canonical"]` |
| `--strict`                    | ✅     | Turn warnings into errors |

## Configuration layers

The effective configuration is built in this order (later overrides earlier):

1. `ConversionOptions::default()`
2. Discovered user config (`$CONFIG_DIR/htmlmd/config.toml`)
3. Discovered project config (`.htmlmd.toml`)
4. Explicit `--config` file
5. Environment variables (`HTMLMD_*`, nested keys separated by `__`)
6. CLI flags

All options are validated before any file is processed.

## Removed options

The following options were accepted and documented in earlier pre-1.0
releases but never affected output. They were removed from the schema rather
than left silently ignored. Configuration files that still contain these keys
load without error — unknown keys are ignored by the loader — but the keys
have no effect.

- `[render]`: `ordered-list-marker`, `emphasis-marker`, `strong-marker`,
  `code-fence-min-length`, `line-wrapping`, `escaping-mode`,
  `character-entities`, `smart-punctuation`, `blank-line-compaction`,
  `url-escaping`, `autolink-detection`, `email-handling`, `comment-policy`,
  `doctype-policy`
- `[semantic]`: `normalize-headings`, `list-indent`, `inline-style-subset`,
  `semantic-tags`, `embedded-media`
- `[extension]`: `rule-packs`

Two formerly inert options were wired instead of removed: `heading-offset`
and `task-lists` (see the `[semantic]` table above).

Enum *variants* that behaved as no-ops were removed too. Unlike unknown keys,
an unknown variant is a load error, so a config using one must be updated:

- `heading-style = "keep"` — silently rendered ATX. Use `"atx"`.
- `form-handling = "checklist"` — did nothing. Use `"readable"` or `"drop"`.

And one was made real rather than removed: `math.output = "block-dollar"` was
byte-identical to `"inline-dollar"`; it now emits `$$…$$` for inline math as
its name implies.
