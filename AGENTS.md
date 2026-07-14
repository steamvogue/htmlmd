# Agent notes

This repository contains `htmlmd`, a Rust HTML-to-Markdown converter.

Before helping with conversion tasks, consult the project-specific skill files:

- `.kimi/skills/htmlmd/SKILL.md` (Kimi)
- `.claude/skills/htmlmd/SKILL.md` (Claude / Codex)
- `.cursorrules` (Cursor)

They cover how to use the CLI, profiles, config files, and HTTP API server.

Additional documentation:

- `docs/PROFILES.md`
- `docs/API_AND_WEB_SERVICE.md`
- `docs/BUILD_AND_DEPLOY.md`
- `docs/PACKAGING.md`
- `docs/OPTION_REFERENCE.md`

Compiled binaries are kept in the `releases/` directory:

- `releases/htmlmd`
- `releases/htmlmd-server`

If they are missing, build them with:

```bash
cargo build -p htmlmd-cli --release
cargo build -p htmlmd-server --release
```
