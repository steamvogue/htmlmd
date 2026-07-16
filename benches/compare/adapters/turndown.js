// Turndown (GFM plugin) adapter: file argument -> Markdown on stdout.
const fs = require("fs");
const TurndownService = require("turndown");
const { gfm } = require("turndown-plugin-gfm");

const td = new TurndownService({ headingStyle: "atx", codeBlockStyle: "fenced" });
td.use(gfm);

const html = fs.readFileSync(process.argv[2], "utf8");
process.stdout.write(td.turndown(html));
