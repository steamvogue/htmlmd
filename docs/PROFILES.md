# Using `htmlmd` profiles

`htmlmd` uses **output profiles** to choose which Markdown dialect and features to emit. A profile is a high-level preset; you can still override any individual option with CLI flags or a config file.

## Available profiles

| Profile | Description | Typical use |
|---------|-------------|-------------|
| `commonmark` | Standard CommonMark output. | Generic Markdown, maximum portability. |
| `gfm` | GitHub Flavored Markdown (tables, task lists, strikethrough, autolinks). | GitHub/GitLab repos, GFM renderers. |
| `extended` | Footnotes, definition lists, math, GitHub alerts, mermaid, semantic tags. | Static sites, note-taking tools. |
| `pandoc` | Like `extended`, but raw HTML is preserved and smart punctuation is normalized. | Pandoc workflows. |
| `obsidian` | `extended` features plus YAML frontmatter and Obsidian-style `[[wikilinks]]`. | Obsidian vaults. |
| `mdx-safe` | `extended` features with raw HTML dropped/escaped and JSX braces escaped. | MDX/Next.js/Docusaurus. |
| `plain-text` | Readable plain text with Markdown markup stripped and images replaced by alt text. | Search indexes, summaries. |

## Selecting a profile

```bash
htmlmd --profile <name> input.html
```

Profiles also enable matching semantic features automatically. For example, `--profile gfm` turns on GFM table handling, and `--profile extended` enables footnotes, definition lists and inline math.

## Profile examples

All examples assume you are in the project root and use the files in `fixtures/`.

### CommonMark

```bash
htmlmd --profile commonmark fixtures/basic.html
```

Output:

```markdown
# Hello World

This is a **bold** and *italic* paragraph.

-   First item
-   Second item

[Example link](https://example.com/)
```

### GFM (GitHub Flavored Markdown)

```bash
htmlmd --profile gfm fixtures/table.html
```

Output:

```markdown
| Language | Type        |
| -------- | ----------- |
| Rust     | Systems     |
| Python   | Interpreted |
```

### Extended

```bash
htmlmd --profile extended fixtures/extended.html
```

Output:

```markdown
This is ==important==, ~~removed~~, and ++added++.

H~2~O and E=mc^2^.

Press <kbd>Ctrl</kbd> + <kbd>C</kbd>.

A footnote reference[^1].

[^1]: Footnote text.

Term
: Definition text.

[another note](<Another page>)

Inline math:$E=mc^2$.

> [!NOTE]
> This is an alert.
```

### Obsidian

The Obsidian profile emits YAML frontmatter when metadata extraction is requested, and converts `<a class="wikilink">` links to `[[Target|text]]` syntax.

```bash
htmlmd --profile obsidian \
       --metadata-title --metadata-description \
       fixtures/extended.html
```

Output:

```markdown
---
title: Extended fixture
description: Demonstrates extended Markdown features
---
This is ==important==, ~~removed~~, and ++added++.

H~2~O and E=mc^2^.

Press <kbd>Ctrl</kbd> + <kbd>C</kbd>.

A footnote reference[^1].

[^1]: Footnote text.

Term
: Definition text.

[[Another page|another note]]

Inline math:$E=mc^2$.

> [!NOTE]
> This is an alert.
```

### Pandoc

Pandoc mode preserves raw HTML so you can post-process with Pandoc, and enables smart-punctuation normalization.

```bash
htmlmd --profile pandoc fixtures/extended.html
```

Output contains raw HTML such as:

```markdown
<div class="footnotes">
&#10;<li id="fn-1">Footnote text. <a href="#fnref-1">↩</a></li>
&#10;</div>
```

### MDX-safe

Use this when the output will be consumed by MDX. Raw HTML is dropped/escaped and `{` `}` braces are escaped so they are not interpreted as JSX expressions.

```bash
htmlmd --profile mdx-safe fixtures/extended.html
```

Output:

```markdown
This is ==important==, ~~removed~~, and ++added++.

H~2~O and E=mc^2^.

Press Ctrl + C.

A footnote reference[^1].

[^1]: Footnote text.

Term
: Definition text.

[another note](<Another page>)

Inline math:$E=mc^2$.

Note

This is an alert.
```

### Plain text

Plain text removes Markdown markup and converts images to their alt text. Useful for full-text search or summaries.

```bash
htmlmd --profile plain-text fixtures/basic.html
```

Output:

```text
Hello World

This is a bold and italic paragraph.

First item
Second item

Example link
```

## Combining profiles with other options

Profiles set sensible defaults, but you can override individual behaviors.

### Extract metadata

```bash
htmlmd --profile obsidian \
       --metadata-title --metadata-description --metadata-canonical-url \
       fixtures/metadata.html
```

### Reference-style links

```bash
# Definitions placed at the end of the document (default for reference style)
htmlmd --link-style reference fixtures/links.html

# Definitions placed immediately after each link
htmlmd --link-style reference --reference-placement adjacent fixtures/links.html

# Definitions placed at the end of the current heading section
htmlmd --link-style reference --reference-placement section-end fixtures/links.html
```

### Image modes

```bash
# Inline images (default)
htmlmd --image-mode inline fixtures/image_mode.html

# Reference-style images
htmlmd --image-mode reference fixtures/image_mode.html

# Drop images entirely
htmlmd --image-mode skip fixtures/image_mode.html

# Keep only the alt text
htmlmd --image-mode alt-text fixtures/image_mode.html
```

### Config file equivalent

Save the settings to `htmlmd.toml` and run `htmlmd --config htmlmd.toml ...`:

```toml
profile = "obsidian"

[render]
link-style = "reference"
reference-placement = "adjacent"

[cleanup]
image-mode = "reference"

[semantic]
footnotes = true
definition-lists = true
```

## Tips

- Use `--print-default-config` to see every option and its default value.
- Use `--print-effective-config` to see the merged config after applying files, environment variables and CLI flags.
- When a profile does not behave exactly as you need, override the specific option instead of abandoning the profile.
