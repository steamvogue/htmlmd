#!/usr/bin/env bash
# Cross-tool HTML→Markdown CLI benchmark.
#
# Measures END-TO-END CLI time (including interpreter/process startup — the
# honest cross-language comparison for command-line use). In-process library
# numbers for the Rust crates live in the criterion bench instead
# (crates/htmlmd-core/benches/convert_bench.rs).
#
# Tools are auto-detected; missing ones are skipped with a note. Expected
# locations (see README.md in this directory for setup):
#   turndown      node + ./node_modules (npm install turndown turndown-plugin-gfm)
#   markdownify   ./.tools/venv (python3 -m venv + pip install markdownify)
#   html2markdown ~/go/bin or PATH (go install .../html-to-markdown/v2/cli/html2markdown@latest)
#   pandoc        ./.tools/pandoc-*/bin or PATH
set -euo pipefail
cd "$(dirname "$0")"
ROOT="$(cd ../.. && pwd)"

command -v hyperfine >/dev/null || { echo "hyperfine is required" >&2; exit 1; }

RESULTS_DIR="results"
CORPUS_DIR="corpus"
mkdir -p "$RESULTS_DIR"

echo "==> Building htmlmd (release) and dumping corpus"
(cd "$ROOT" && cargo build --release -p htmlmd-cli >/dev/null)
(cd "$ROOT" && cargo run --release -q -p htmlmd-core --example dump_corpus -- "benches/compare/$CORPUS_DIR")

TARGET_SUBDIR="${CARGO_BUILD_TARGET:+${CARGO_BUILD_TARGET}/}"
HTMLMD="$ROOT/target/${TARGET_SUBDIR}release/htmlmd"

H2M="$(command -v html2markdown || echo "$HOME/go/bin/html2markdown")"
PANDOC="$(command -v pandoc || ls .tools/pandoc-*/bin/pandoc 2>/dev/null | head -1 || true)"

for doc in wiki news docs tables; do
    input="$CORPUS_DIR/$doc.html"
    cmds=(--command-name "htmlmd" "$HTMLMD --profile gfm $input")

    if [ -f node_modules/turndown/package.json ] && command -v node >/dev/null; then
        cmds+=(--command-name "turndown" "node adapters/turndown.js $input")
    else
        echo "note: turndown skipped (npm install turndown turndown-plugin-gfm)" >&2
    fi
    if [ -x .tools/venv/bin/python ]; then
        cmds+=(--command-name "markdownify" ".tools/venv/bin/python adapters/markdownify_adapter.py $input")
    else
        echo "note: markdownify skipped (.tools/venv missing)" >&2
    fi
    if [ -x "$H2M" ]; then
        cmds+=(--command-name "html2markdown-v2" "$H2M --plugin-table --plugin-strikethrough <$input")
    else
        echo "note: html2markdown skipped (go install ...)" >&2
    fi
    if [ -n "$PANDOC" ] && [ -x "$PANDOC" ]; then
        cmds+=(--command-name "pandoc" "$PANDOC -f html -t gfm --wrap=none $input")
    else
        echo "note: pandoc skipped" >&2
    fi

    echo "==> $doc ($(stat -c%s "$input") bytes)"
    hyperfine --warmup 2 --min-runs 5 --output /dev/null \
        --export-json "$RESULTS_DIR/$doc.json" "${cmds[@]}"
done

echo
python3 summarize.py "$RESULTS_DIR"
