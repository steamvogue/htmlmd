# Development notes

Internal notes for people (and AI assistants) working on `htmlmd` itself.
If you just want to use the tool, start with the [README](../README.md).

## Project status

**Phase 3 is complete.** The workspace, library API, `htmd`-backed conversion
engine, CLI, config loading, fixtures, and the major Phase 3 features are
implemented and tested:

- Output profiles: `commonmark`, `gfm`, `extended`, `pandoc`, `obsidian`, `mdx-safe`, `plain-text`
- Extended Markdown: footnotes, definition lists, math, GitHub-style alerts, mermaid diagrams
- Semantic tag handling: `mark`, `del`, `ins`, `sub`, `sup`, `kbd`, etc.
- Advanced tables: GFM, HTML fallback, CSV-like, flatten, drop strategies
- Code-block language detection from classes and source heuristics
- Custom per-selector rules (`drop`, `unwrap`, `text`, `html`, `markdown-template`, `fenced-block`, `link`, `image`)
- Image modes: inline, skip, alt-text, reference
- Reference link placement: end, adjacent, section-end
- DOM/output safety limits

See [`OPTION_REFERENCE.md`](OPTION_REFERENCE.md) for the implementation status
of every option.

## Workspace layout

```text
.
├── Cargo.toml
├── crates/
│   ├── htmlmd-core/      # Reusable library (benches in crates/htmlmd-core/benches/)
│   ├── htmlmd-cli/       # `htmlmd` binary
│   └── htmlmd-server/    # `htmlmd-server` HTTP API binary
├── fixtures/             # HTML fixtures and expected Markdown
├── scripts/              # build-release.sh and friends
├── dist/                 # Per-target-triple release binaries (gitignored)
└── docs/
```

## Build, test, lint, bench

```bash
cargo build --workspace --release
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo bench -p htmlmd-core --bench convert_bench
```

Release binaries go into `dist/<target-triple>/` via
`scripts/build-release.sh` — see [`BUILD_AND_DEPLOY.md`](BUILD_AND_DEPLOY.md).

## LLM skill files

This repository includes project-specific instructions for AI coding
assistants. Keep these in sync when you change the tool's behavior, flags, or
API.

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

To add support for a new assistant, create its standard skill file in this
repo (for example, `.copilot/skills/htmlmd/SKILL.md`) and point to it from
`AGENTS.md`.
