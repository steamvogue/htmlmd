# Cross-tool benchmark harness

Reproducible end-to-end CLI comparison of `htmlmd` against other
HTML→Markdown converters on the deterministic synthetic corpus
(`htmlmd-core`'s `corpus` module — identical bytes for every tool).

Measures **full CLI invocation time** with [hyperfine], including process and
interpreter startup — the honest comparison for command-line use. In-process
library numbers for the Rust crates (htmd, fast_html2md, mdka) live in the
criterion bench instead: `cargo bench -p htmlmd-core --bench convert_bench`.

## Setup (each tool optional; missing ones are skipped)

```bash
# turndown (Node)
npm install turndown turndown-plugin-gfm          # in this directory

# markdownify (Python)
python3 -m venv .tools/venv && .tools/venv/bin/pip install markdownify

# html-to-markdown v2 (Go)
go install github.com/JohannesKaufmann/html-to-markdown/v2/cli/html2markdown@latest

# pandoc: any install on PATH, or unpack a release tarball into .tools/
```

## Run

```bash
./run.sh          # results/*.json + Markdown summary on stdout
```

Fairness notes: every tool is asked for GFM-style output where it has a
flag for it (htmlmd `--profile gfm`, turndown + gfm plugin, html2markdown
`--plugin-table --plugin-strikethrough`, pandoc `-t gfm`, markdownify ATX
headings). Output *quality* differs — this harness measures speed only;
see docs/BENCHMARKS.md for the feature-set comparison context.

[hyperfine]: https://github.com/sharkdp/hyperfine
