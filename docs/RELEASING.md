# Releasing `htmlmd`

Short runbook for cutting a release. Binaries and the Docker image are
automated; crates.io publishing is manual.

## 1. Bump versions

The whole workspace shares one version. Update it in two places in the root
`Cargo.toml`:

- `[workspace.package] version`
- the `version` on the `htmlmd-core` entry in `[workspace.dependencies]`
  (must match, or publishing `htmlmd-cli` will pull the wrong core release)

Then refresh the lockfile and sanity-check:

```bash
cargo check --workspace --locked || cargo update -w
cargo test --workspace
```

## 2. Update the changelog

`CHANGELOG.md` keeps an `## [Unreleased]` section. Before tagging, retitle it
`## [X.Y.Z] - YYYY-MM-DD` and open a fresh empty `## [Unreleased]` above it.
The tag and the changelog heading must agree — the release notes are
generated from commits, so the changelog is the only curated record.

## 3. Tag and push

```bash
git tag vX.Y.Z
git push --tags
```

## 4. What the tag triggers

Pushing a `v*` tag runs [`.github/workflows/release.yml`](../.github/workflows/release.yml).
It first runs the full test suite on ubuntu, macos and windows — a tag that
points at a commit ci.yml never saw does not ship. It then builds every
target and creates a **draft** GitHub Release (with generated notes)
containing, per target:

- `htmlmd-vX.Y.Z-<triple>.tar.gz` (`.zip` for Windows) with `htmlmd`,
  `htmlmd-server` and `SHA256SUMS` at the archive root — same layout as
  `scripts/build-release.sh` produces locally in `dist/<triple>/`
- targets: linux x86_64 gnu + musl, linux aarch64 gnu + musl, macOS aarch64
  (Apple Silicon), Windows x86_64 MSVC

The `gnu` Linux builds link against the build runner's glibc — 2.39 on the
Ubuntu 24.04 images — so they will not start on an older distro. Raspberry Pi
OS Bookworm is glibc 2.36, so **Pi users want the `aarch64-unknown-linux-musl`
archive**, which is statically linked and has no such floor. Same reasoning
for `x86_64-unknown-linux-musl` on older x86 distros. Point people at the musl
asset whenever they report a `GLIBC_2.xx not found` error.

The Windows build sets `-C target-feature=+crt-static` (via `matrix.rustflags`)
so the exe carries its C runtime instead of importing `VCRUNTIME140.dll`, which
comes from the VC++ redistributable and is absent on a clean Windows. The flag
lives in the workflow rather than `.cargo/config.toml` on purpose: config
rustflags reach proc macros on a host build, `crt-static` forbids the dylib
crate type proc macros need, and `serde_derive`/`clap_derive` would stop
building — taking plain `cargo build` and ci.yml on Windows with them. It is
safe in the workflow only because every build there passes `--target`, which
keeps host and target flags separate. **If you ever drop `--target`, drop this
flag too.**

and pushes a multi-arch (amd64/arm64) Docker image to
`ghcr.io/steamvogue/htmlmd-server:vX.Y.Z`, plus `:latest` for stable tags
only — a prerelease tag (anything with a `-`, e.g. `v0.2.0-rc.1`) never
becomes `:latest`.

**The binaries wait as a draft until you publish them; the Docker image does
not.** An image push is public the moment it lands on ghcr.io — there is no
draft state for registries — so the test gate is the only thing standing
between a bad tag and a pullable image. Nothing downloads a draft release
though, which also means `cargo binstall` fails until the release is
published.

These archives are what `cargo binstall htmlmd-cli` downloads (see
`[package.metadata.binstall]` in `crates/htmlmd-cli/Cargo.toml`) — if you
rename the assets, update that template too.

## 5. Publish to crates.io (manual)

Needs a crates.io token (`cargo login`). Publish core first — the CLI
depends on it by version:

```bash
cargo publish -p htmlmd-core
cargo publish -p htmlmd-cli
```

Dry-run first if in doubt: `cargo publish --dry-run -p htmlmd-core`.

## 6. Inspect the draft, then publish

- On the GitHub Releases page: six archives plus generated notes. Download
  one or two, check `SHA256SUMS`, run `htmlmd --version`.
- Publish the release. Only now do the assets (and `cargo binstall`) go live.
- `docker run --rm -p 3000:3000 ghcr.io/steamvogue/htmlmd-server:vX.Y.Z`
  and hit `http://localhost:3000/health` as a smoke test.
- For winget/apt distribution, see [`docs/PACKAGING.md`](PACKAGING.md).
