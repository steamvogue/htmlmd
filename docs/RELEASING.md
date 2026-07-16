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

`CHANGELOG.md` does not exist yet (planned in ROADMAP M5). Once it does,
add the release section here before tagging.

## 3. Tag and push

```bash
git tag vX.Y.Z
git push --tags
```

## 4. What the tag triggers

Pushing a `v*` tag runs [`.github/workflows/release.yml`](../.github/workflows/release.yml),
which creates a GitHub Release (with generated notes) containing, per target:

- `htmlmd-vX.Y.Z-<triple>.tar.gz` (`.zip` for Windows) with `htmlmd`,
  `htmlmd-server` and `SHA256SUMS` at the archive root — same layout as
  `scripts/build-release.sh` produces locally in `dist/<triple>/`
- targets: linux x86_64 gnu + musl, linux aarch64 gnu, macOS aarch64
  (Apple Silicon), Windows x86_64 MSVC

and pushes a multi-arch (amd64/arm64) Docker image to
`ghcr.io/steamvogue/htmlmd-server:vX.Y.Z` and `:latest`.

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

## 6. Afterwards

- Verify the release assets and checksums on the GitHub Releases page.
- `docker run --rm -p 3000:3000 ghcr.io/steamvogue/htmlmd-server:vX.Y.Z`
  and hit `http://localhost:3000/health` as a smoke test.
- For winget/apt distribution, see [`docs/PACKAGING.md`](PACKAGING.md).
