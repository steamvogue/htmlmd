# Packaging `htmlmd` for winget and apt-get

This guide covers creating installable packages for Windows (winget) and Debian/Ubuntu (apt-get). You should already have a release binary; see [`docs/BUILD_AND_DEPLOY.md`](BUILD_AND_DEPLOY.md) if you need build instructions.

> **Note:** tagged releases already publish prebuilt archives
> (`htmlmd-<tag>-<triple>.tar.gz`, `.zip` on Windows) and a Docker image via
> [`.github/workflows/release.yml`](../.github/workflows/release.yml) — see
> [`docs/RELEASING.md`](RELEASING.md). The steps below cover the extra,
> currently manual distribution channels.

## Rust users: crates.io and cargo-binstall

Once the crates are published (see [`docs/RELEASING.md`](RELEASING.md)),
Rust users can install straight from crates.io:

```bash
cargo install htmlmd-cli          # builds from source
cargo binstall htmlmd-cli        # downloads the prebuilt release archive
```

`cargo binstall` needs no extra setup on our side: the
`[package.metadata.binstall]` section in `crates/htmlmd-cli/Cargo.toml`
maps each target triple to the matching GitHub Release asset.

## Windows: winget

winget packages are described by YAML manifests in the [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) repository. You can submit manifests with the `wingetcreate` tool or by hand.

### 1. Create an installer

The easiest installer for a Rust CLI is an MSI created with [`cargo-wix`](https://github.com/volks73/cargo-wix).

```powershell
# On Windows
cargo install cargo-wix
cargo wix -p htmlmd-cli --output target/wix/htmlmd.msi
```

If you prefer a simple portable archive, the release workflow already
uploads `htmlmd-<tag>-x86_64-pc-windows-msvc.zip` to every GitHub Release,
so there is nothing to build — point the manifest at that asset. To produce
one by hand instead:

```powershell
Compress-Archive -Path target\release\htmlmd.exe -DestinationPath target\htmlmd-x86_64-pc-windows-msvc.zip
```

Upload the installer or archive to a public URL (for example, a GitHub Release asset).

### 2. Install `wingetcreate`

```powershell
winget install Microsoft.WingetCreate
```

### 3. Generate and submit the manifest

Interactive mode:

```powershell
wingetcreate new
```

Non-interactive example:

```powershell
wingetcreate new `
  --urls https://github.com/steamvogue/htmlmd/releases/download/v0.1.0/htmlmd.msi `
  --version 0.1.0
```

`wingetcreate` will calculate the SHA256 hash, ask for package metadata, and then open a pull request to `microsoft/winget-pkgs`. After the PR is merged, users can install with:

```powershell
winget install htmlmd
```

### 4. Manual manifest example

If you prefer to write the manifest yourself, create a file like `h/htmlmd/0.1.0/htmlmd.installer.yaml`:

```yaml
PackageIdentifier: htmlmd.htmlmd
PackageVersion: 0.1.0
InstallerType: wix
Installers:
  - Architecture: x64
    InstallerUrl: https://github.com/steamvogue/htmlmd/releases/download/v0.1.0/htmlmd.msi
    InstallerSha256: <sha256>
ManifestType: installer
ManifestVersion: 1.6.0
```

And a matching `htmlmd.locale.en-US.yaml` and `htmlmd.yaml`. Submit all three files in a PR to `microsoft/winget-pkgs` under `manifests/h/htmlmd/0.1.0/`.

## Linux: apt-get

The standard way to distribute a Rust CLI on Debian/Ubuntu is a `.deb` package. You can build one with [`cargo-deb`](https://github.com/kornelski/cargo-deb) and host it in an APT repository.

### 1. Build the `.deb`

```bash
cargo install cargo-deb
cargo deb -p htmlmd-cli
```

The package is written to:

```text
target/debian/htmlmd_0.1.0_amd64.deb
```

You can inspect it with:

```bash
dpkg -I target/debian/htmlmd_0.1.0_amd64.deb
dpkg -c target/debian/htmlmd_0.1.0_amd64.deb
```

### 2. Optional: customize Debian metadata

Add a `[package.metadata.deb]` section to `crates/htmlmd-cli/Cargo.toml` if you want more control:

```toml
[package.metadata.deb]
maintainer = "htmlmd contributors <htmlmd@example.com>"
copyright = "2026, htmlmd contributors"
extended-description = """\
  A fast, configurable HTML-to-Markdown converter \
  with multiple output profiles."""
section = "utils"
priority = "optional"
assets = [
    ["target/release/htmlmd", "usr/bin/", "755"],
    ["README.md", "usr/share/doc/htmlmd/README", "644"],
]
```

### 3. Host an APT repository

A minimal self-hosted repository uses `reprepro` and a GPG key.

Generate or use an existing GPG key:

```bash
gpg --full-generate-key
# Note the key ID, e.g. 1234ABCD
```

Create the repository structure:

```bash
mkdir -p apt-repo/{conf,dists,pool}
cat > apt-repo/conf/distributions <<'EOF'
Origin: htmlmd
Label: htmlmd
Suite: stable
Codename: stable
Architectures: amd64 arm64
Components: main
Description: htmlmd APT repository
SignWith: 1234ABCD
EOF
```

Add the package:

```bash
reprepro -b apt-repo includedeb stable target/debian/htmlmd_0.1.0_amd64.deb
```

Serve `apt-repo/` with any static web server, for example nginx:

```nginx
server {
    listen 80;
    server_name apt.example.com;
    root /var/www/apt-repo;
    autoindex off;
}
```

### 4. Install on a client machine

```bash
# Import the repository public key
curl -fsSL https://apt.example.com/htmlmd.gpg | sudo gpg --dearmor -o /usr/share/keyrings/htmlmd.gpg

# Add the source list
echo "deb [signed-by=/usr/share/keyrings/htmlmd.gpg] https://apt.example.com stable main" \
  | sudo tee /etc/apt/sources.list.d/htmlmd.list

# Install
sudo apt update
sudo apt install htmlmd
```

### 5. Alternative: Launchpad PPA

For Ubuntu users, a [Launchpad PPA](https://help.launchpad.net/Packaging/PPA) is often more convenient than self-hosting. You will need:

1. A `debian/` source package directory.
2. A Launchpad account and GPG key.
3. A recipe that builds from your Git repository.

After the PPA is published, users install with:

```bash
sudo add-apt-repository ppa:yourusername/htmlmd
sudo apt update
sudo apt install htmlmd
```

## Summary

| Platform | Package format | Tooling | Public index |
|----------|----------------|---------|--------------|
| Rust toolchain | crate / prebuilt archive | `cargo install`, `cargo binstall` | crates.io + GitHub Releases |
| Windows | `.msi` or `.zip` | `cargo-wix`, `wingetcreate` | `microsoft/winget-pkgs` |
| Debian/Ubuntu | `.deb` | `cargo-deb`, `reprepro` | Self-hosted apt repo or Launchpad PPA |

Cross-platform binaries and the Docker image are produced automatically by
[`.github/workflows/release.yml`](../.github/workflows/release.yml) on every
`v*` tag; the runbook lives in [`docs/RELEASING.md`](RELEASING.md).
