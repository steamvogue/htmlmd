# Building and deploying `htmlmd`

`htmlmd` is a Rust workspace. The main binary lives in `crates/htmlmd-cli` and the conversion library in `crates/htmlmd-core`.

## Requirements

- **Rust 1.85 or later** (the workspace `rust-version`).
- A C compiler toolchain for your target platform.
- (Optional) `git` to clone the repository.

The instructions below use `cargo`. If you do not have Rust installed, get it from <https://rustup.rs>.

## Linux build

### 1. Install dependencies (Debian/Ubuntu)

```bash
sudo apt update
sudo apt install -y build-essential curl git
```

On Fedora/RHEL:

```bash
sudo dnf install -y gcc git
```

### 2. Clone and build

```bash
git clone https://github.com/steamvogue/htmlmd.git
cd htmlmd
cargo build --workspace --release
```

### 3. Locate the binary

```bash
ls -l target/release/htmlmd
```

### 4. Install locally

```bash
# Installs into ~/.cargo/bin (make sure it is on your PATH)
cargo install --path crates/htmlmd-cli

htmlmd --version
```

### 5. Install system-wide

```bash
sudo cp target/release/htmlmd /usr/local/bin/
sudo chmod +x /usr/local/bin/htmlmd
htmlmd --version
```

## Windows build

### On Windows with MSVC

1. Install the **Visual Studio Build Tools** or Visual Studio with the **Desktop development with C++** workload.
2. Install Rust with the default `x86_64-pc-windows-msvc` toolchain.
3. Open a terminal (PowerShell or Command Prompt) and run:

```powershell
git clone https://github.com/steamvogue/htmlmd.git
cd htmlmd
cargo build --workspace --release
```

The binary is at:

```text
target\release\htmlmd.exe
```

### Cross-compiling from Linux to Windows

You can build a Windows GNU binary from any Linux host — including an ARM64
one, targeting x86-64 Windows — using MinGW. The linker is already configured
in [`.cargo/config.toml`](../.cargo/config.toml), so you only need the
toolchain:

```bash
sudo apt install -y gcc-mingw-w64-x86-64   # or the full mingw-w64
rustup target add x86_64-pc-windows-gnu
```

Build:

```bash
scripts/build-release.sh x86_64-pc-windows-gnu
# or: cargo build --workspace --release --target x86_64-pc-windows-gnu
```

The script drops both binaries plus `SHA256SUMS` into
`dist/x86_64-pc-windows-gnu/`; a plain cargo build leaves them at
`target/x86_64-pc-windows-gnu/release/htmlmd.exe`.

The result is a self-contained `PE32+` executable: it imports only Windows
system DLLs (`kernel32`, `ntdll`, `bcryptprimitives`, …), so there is no
`libgcc`/`libwinpthread` runtime to copy alongside it. `mimalloc`, the one
C dependency, cross-compiles cleanly.

> **Note:** The native Windows MSVC target (`x86_64-pc-windows-msvc`) can only be built on Windows with the MSVC toolchain. Use the GNU target above for Linux-hosted cross-compilation.

## macOS build

```bash
git clone https://github.com/steamvogue/htmlmd.git
cd htmlmd
cargo build --workspace --release
```

The binary is at `target/release/htmlmd`.

## Release build tips

Use the release profile for optimized binaries:

```bash
cargo build --workspace --release --locked
```

Strip debug symbols to reduce size:

```bash
# Linux / macOS
strip target/release/htmlmd

# Windows (using LLVM strip from the MSVC toolchain)
llvm-strip target/release/htmlmd.exe
```

## Per-architecture output layout (`dist/`)

Local release binaries live under `dist/<target-triple>/`, one directory per
architecture, so builds for different machines never overwrite each other and
never show up in `git status` (the whole `dist/` tree is gitignored):

```text
dist/
├── aarch64-unknown-linux-gnu/
│   ├── htmlmd
│   ├── htmlmd-server
│   └── SHA256SUMS
└── x86_64-unknown-linux-gnu/
    ├── htmlmd
    ├── htmlmd-server
    └── SHA256SUMS
```

`scripts/build-release.sh` builds, strips, copies, and checksums in one step:

```bash
# Host architecture
scripts/build-release.sh

# Explicit targets (must be installed via `rustup target add`)
scripts/build-release.sh aarch64-unknown-linux-gnu x86_64-pc-windows-gnu
```

Always passing `--target <triple>` (the script does this even for the host)
also keeps cargo's own artifacts separated under `target/<triple>/release/`,
so a native build and a cross build never invalidate each other's caches.

## Deployment options

### Manual deployment

Copy the binary to every target machine and place it on the system `PATH`.

```bash
# Linux / macOS
sudo install -Dm755 target/release/htmlmd /usr/local/bin/htmlmd

# Windows: add target\release\htmlmd.exe to PATH or install to C:\Tools
```

### GitHub Actions release automation

Releases are fully automated: pushing a `v*` tag runs
[`.github/workflows/release.yml`](../.github/workflows/release.yml), which
builds `htmlmd` and `htmlmd-server` for:

| Target | Runner |
|--------|--------|
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` |
| `aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` |
| `x86_64-unknown-linux-musl` | `ubuntu-latest` (+ musl-tools) |
| `aarch64-apple-darwin` | `macos-14` |
| `x86_64-pc-windows-msvc` | `windows-latest` |

Each target is assembled in the same `dist/<triple>/` layout that
`scripts/build-release.sh` produces locally (both binaries plus
`SHA256SUMS`) and uploaded to the GitHub Release as
`htmlmd-<tag>-<triple>.tar.gz` (`.zip` on Windows). The workflow also
builds and pushes a multi-arch (amd64/arm64) server image to
`ghcr.io/steamvogue/htmlmd-server`. See [`docs/RELEASING.md`](RELEASING.md)
for the step-by-step release runbook.

### Docker deployment

The HTTP server ships with a real Dockerfile at
[`crates/htmlmd-server/Dockerfile`](../crates/htmlmd-server/Dockerfile).
Build it from the **workspace root** (the build context must contain the
whole workspace, not just the server crate):

```bash
docker build -f crates/htmlmd-server/Dockerfile -t htmlmd-server .
docker run --rm -p 3000:3000 htmlmd-server
```

The image listens on `0.0.0.0:3000` (set via the `HTMLMD_BIND` environment
variable; override with `-e HTMLMD_BIND=...`). Tagged releases publish
multi-arch (amd64/arm64) images, so you can skip building entirely:

```bash
docker run --rm -p 3000:3000 ghcr.io/steamvogue/htmlmd-server:latest
```

A minimal image for the CLI follows the same pattern:

```dockerfile
FROM rust:1.94-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build -p htmlmd-cli --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/htmlmd /usr/local/bin/htmlmd
ENTRYPOINT ["htmlmd"]
```

Build and run:

```bash
docker build -t htmlmd .
docker run --rm -v "$PWD/fixtures:/data" htmlmd /data/basic.html
```

## Next steps

- For the tag-to-release runbook (versions, tagging, crates.io publishing), see [`docs/RELEASING.md`](RELEASING.md).
- For packaging the binaries into installable formats (`.msi`, `.deb`, winget, apt repositories), see [`docs/PACKAGING.md`](PACKAGING.md).
