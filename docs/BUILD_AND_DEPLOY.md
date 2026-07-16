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

You can build a Windows GNU binary from Linux using MinGW.

```bash
sudo apt install -y mingw-w64
rustup target add x86_64-pc-windows-gnu
```

Tell Cargo which linker to use by creating `.cargo/config.toml` in the project root:

```toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
```

Build:

```bash
cargo build --workspace --release --target x86_64-pc-windows-gnu
```

The binary is at:

```text
target/x86_64-pc-windows-gnu/release/htmlmd.exe
```

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

### GitHub Actions release matrix

A typical CI job builds for multiple platforms and uploads the binaries as release artifacts:

```yaml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            binary: htmlmd
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            binary: htmlmd.exe
          - os: macos-latest
            target: aarch64-apple-darwin
            binary: htmlmd
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --workspace --release --target ${{ matrix.target }} --locked
      - uses: actions/upload-artifact@v4
        with:
          name: htmlmd-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/${{ matrix.binary }}
```

### Docker deployment

A minimal image can run `htmlmd` as a command-line tool:

```dockerfile
FROM rust:1.94 AS builder
WORKDIR /app
COPY . .
RUN cargo build --workspace --release

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

For packaging the binaries into installable formats (`.msi`, `.deb`, winget, apt repositories), see [`docs/PACKAGING.md`](PACKAGING.md).
