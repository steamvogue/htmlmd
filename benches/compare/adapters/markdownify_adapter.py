# markdownify adapter: file argument -> Markdown on stdout.
import sys

from markdownify import markdownify

with open(sys.argv[1], encoding="utf8") as f:
    html = f.read()
sys.stdout.write(markdownify(html, heading_style="ATX"))
