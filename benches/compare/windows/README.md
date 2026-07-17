# Windows benchmark kit

The Windows counterpart of [`../run.sh`](../run.sh). Same corpus, same
fairness rules, same output format — see [`../README.md`](../README.md) for
what is being measured and why.

## Use

```powershell
# 1. Fetch the tools (no admin, everything lands in .\tools\)
powershell -ExecutionPolicy Bypass -File setup.ps1

# 2. Copy htmlmd.exe and dump_corpus.exe next to these scripts
#    (or leave them in dist\x86_64-pc-windows-gnu\ — both are checked)

# 3. Run
powershell -ExecutionPolicy Bypass -File run.ps1
```

`run.ps1 -OutputProfile extended` benchmarks a different htmlmd profile.
(Not `-Profile` — that collides with a PowerShell automatic variable.)

Both scripts are deliberately pure ASCII and work under Windows PowerShell
5.1 (`powershell`) as well as PowerShell 7 (`pwsh`); 5.1 reads a
UTF-8-without-BOM script as ANSI, which would mangle any non-ASCII character.

## What setup.ps1 fetches

| Tool | Version | Source |
|---|---|---|
| hyperfine | 1.20.0 | GitHub release (the benchmark driver — required) |
| pandoc | 3.10 | GitHub release |
| html-to-markdown v2 | 2.5.2 | GitHub release |
| Node + turndown + turndown-plugin-gfm | 24.18.0 LTS | nodejs.org zip + npm |
| markdownify | latest | pip, into `tools\venv` — **needs a system Python** |

Versions are pinned so numbers stay comparable between runs; bump them
deliberately and say so when publishing.

Everything is portable: no admin rights, no PATH changes, nothing installed
system-wide. `rm -r tools node_modules corpus results` undoes it completely.
Inside the repo, `node_modules\` lands one level up in `benches\compare\`
instead, shared with `run.sh` — it has to sit next to `adapters\` for Node to
resolve it. Python is the exception — if none is found, markdownify is skipped
and the run simply omits that row.

## Getting the binaries

**Best option: build on the Windows box itself, if Rust is installed there.**
A stock `rustup` toolchain is `x86_64-pc-windows-msvc` — the same ABI the
release workflow ships — so the benchmark measures the artifact people
actually download. From the repo root:

```powershell
cargo build --release
cargo build --release -p htmlmd-core --example dump_corpus
```

`run.ps1` finds both under `target\release\` on its own; nothing to copy.
Confirm with the `htmlmd:` path it prints at startup.

Otherwise, take them from a tagged GitHub release (also
`x86_64-pc-windows-msvc`), or cross-compile the GNU target from a Linux host —
including an ARM64 one:

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install -y gcc-mingw-w64-x86-64
scripts/build-release.sh x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu -p htmlmd-core --example dump_corpus
```

The linker is already configured in [`.cargo/config.toml`](../../../.cargo/config.toml).
The cross-build is `windows-gnu`, a different ABI and C runtime from the
shipped `windows-msvc` binary — fine for tool-vs-tool ratios, but say which
one produced any number you publish.

## Comparing the two ABIs

`windows-msvc` links the UCRT via `VCRUNTIME140.dll`; the mingw `windows-gnu`
cross-build links `msvcrt.dll`. Different C runtime, different allocator
plumbing, so they need not perform alike — and only the msvc one ships. To
measure the gap instead of guessing at it, put both under `dist\<triple>\`
and pass `-CompareBuilds`:

```powershell
mkdir dist\x86_64-pc-windows-msvc
copy target\release\htmlmd.exe        dist\x86_64-pc-windows-msvc\
copy target\release\htmlmd-server.exe dist\x86_64-pc-windows-msvc\

cd benches\compare\windows
powershell -ExecutionPolicy Bypass -File run.ps1 -CompareBuilds
```

Each build becomes its own row (`htmlmd-msvc`, `htmlmd-gnu`) in the same
hyperfine run, against the same corpus on the same machine, with only the
binary differing — the one arrangement where an msvc-vs-gnu number is worth
anything. Ratios are quoted against the first row, `htmlmd-msvc`. A build that
isn't there is reported and skipped.

Bear in mind the three small corpora sit at the Windows process-spawn floor
(see [`docs/BENCHMARKS.md`](../../../docs/BENCHMARKS.md)), so an ABI
difference will only be legible on `wiki`.

## Reading the results

**Windows numbers are not comparable to the Linux numbers in
[`docs/BENCHMARKS.md`](../../../docs/BENCHMARKS.md)** — different machine,
different OS. Publish them under their own machine heading, and record which
ABI the binary used (`windows-gnu` when cross-compiled as above,
`windows-msvc` when taken from a release), because the two can differ in
performance.
