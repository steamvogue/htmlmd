#!/usr/bin/env bash
# Build release binaries into dist/<target-triple>/ so artifacts from
# different architectures never collide and never pollute git status.
#
# Usage:
#   scripts/build-release.sh                 # build for the host target
#   scripts/build-release.sh <triple>...     # build for explicit targets
#
# Examples:
#   scripts/build-release.sh aarch64-unknown-linux-gnu
#   scripts/build-release.sh x86_64-unknown-linux-gnu x86_64-pc-windows-gnu
#
# Cross targets must already be installed (rustup target add <triple>)
# and have a working linker configured in .cargo/config.toml.
set -euo pipefail

cd "$(dirname "$0")/.."

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
TARGETS=("${@:-$HOST_TRIPLE}")
BINARIES=(htmlmd htmlmd-server)

for target in "${TARGETS[@]}"; do
    echo "==> Building ${target}"
    cargo build --workspace --release --locked --target "$target"

    ext=""
    case "$target" in *windows*) ext=".exe" ;; esac

    out_dir="dist/${target}"
    mkdir -p "$out_dir"

    for bin in "${BINARIES[@]}"; do
        src="target/${target}/release/${bin}${ext}"
        cp "$src" "$out_dir/"
        # strip is best-effort: skip when no strip tool exists for the target
        if [[ "$target" == "$HOST_TRIPLE" && -z "$ext" ]]; then
            strip "$out_dir/${bin}" || true
        fi
    done

    (cd "$out_dir" && sha256sum -- "${BINARIES[@]/%/$ext}" >SHA256SUMS)
    echo "==> ${out_dir}:"
    ls -lh "$out_dir"
done
