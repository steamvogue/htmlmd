# Summarize hyperfine JSON exports into a Markdown table.
import json
import sys
from pathlib import Path

results_dir = Path(sys.argv[1] if len(sys.argv) > 1 else "results")
docs = ["wiki", "news", "docs", "tables"]

rows = {}
tools = []
for doc in docs:
    path = results_dir / f"{doc}.json"
    if not path.exists():
        continue
    for r in json.loads(path.read_text())["results"]:
        name = r["command"]
        if name not in tools:
            tools.append(name)
        rows.setdefault(name, {})[doc] = (r["mean"], r["stddev"])

present = [d for d in docs if any(d in rows.get(t, {}) for t in tools)]
baseline = {d: rows.get("htmlmd", {}).get(d, (None,))[0] for d in present}

print("| tool | " + " | ".join(present) + " |")
print("|---|" + "---|" * len(present))
for tool in tools:
    cells = []
    for d in present:
        if d in rows.get(tool, {}):
            mean, std = rows[tool][d]
            rel = f" ({mean / baseline[d]:.1f}x)" if baseline[d] and tool != "htmlmd" else ""
            cells.append(f"{mean * 1000:.0f}±{std * 1000:.0f} ms{rel}")
        else:
            cells.append("—")
    print(f"| {tool} | " + " | ".join(cells) + " |")
