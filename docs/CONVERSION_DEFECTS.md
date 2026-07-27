# Conversion defects found against a drug-label corpus

Six reproducible defects in `htmlmd 0.1.0`, found while converting about 1,200
regulatory documents (DailyMed SPL drug labels, EMC Summaries of Product
Characteristics, MedlinePlus pages) with `--profile plain-text`.

Reported 2026-07-26 by the `astropc` project, which uses htmlmd to turn drug
labels into evidence packs that editorial writers quote from. Its constraint is
unusual and makes these defects sharper than they would be for most consumers:
**no claim may be published without a verbatim quotation from the source**, so
any content htmlmd fails to convert cannot be written about at all, and any text
htmlmd invents can be quoted as fact.

Every defect below has a shell reproduction that needs no corpus. Severity is
from a consumer's point of view: whether the defect silently destroys source
content or silently manufactures content that was never there.

| # | Defect | Severity |
|---|--------|----------|
| 1 | `colspan`/`rowspan` dumps the whole table as raw HTML | high |
| 2 | A table with no `<thead>` loses all row association | high |
| 3 | `<dl>` orphans the term from its definition | medium |
| 4 | `--profile pandoc` passes block elements through raw | low |
| 5 | U+00A0 preserved as a heading separator | medium |
| 6 | `<img alt>` text emitted as body prose | medium |

---

## 1. `colspan` or `rowspan` on a cell dumps the whole table as raw HTML

Any `<td>` or `<th>` carrying `colspan` or `rowspan` makes the enclosing
`<table>` fall back to raw HTML passthrough. **The attribute value is
irrelevant**: `colspan="2"` and `colspan=""` both trigger it. Other empty
attributes do not, and the same attribute on a non-table element does not.

```bash
# passthrough (bug)
echo '<table><tbody><tr><td colspan="">Cell text.</td></tr></tbody></table>' | htmlmd --profile plain-text
echo '<table><tbody><tr><td colspan="2">Cell text.</td></tr></tbody></table>' | htmlmd --profile plain-text
echo '<table><tbody><tr><th rowspan="2">Cell text.</th></tr></tbody></table>'  | htmlmd --profile plain-text
# actual:   <table><tbody><tr><td colspan="">Cell text.</td></tr></tbody></table>
# expected: Cell text.

# converts correctly (control)
echo '<table><tbody><tr><td valign="">Cell text.</td></tr></tbody></table>'   | htmlmd --profile plain-text
echo '<table><tbody><tr><td data-x="">Cell text.</td></tr></tbody></table>'   | htmlmd --profile plain-text
```

In `commonmark`, `gfm` and `extended` the raw HTML is additionally wrapped in a
` ```html ` fence. In `plain-text` it is emitted bare, which is worse: a profile
named plain-text arguably should never emit markup at all.

**Corpus impact:** 717 of ~1,200 converted files contain a raw `<table>`. Merged
header cells are ordinary in real-world tables, so this is not an edge case. The
associated `<span>` (409 files), `<p>` (470), `<br>` (419) and `<a>` (201) leaks
are those same tables' innards, not separate defects.

**Suggested fix:** degrade gracefully. If span attributes cannot be represented
in the target profile, flatten the table to its cell text rather than emitting
source markup.

---

## 2. A table with no `<thead>` loses all row and column association

With only `<tbody>`, every cell becomes its own bare block. Nothing records which
cells shared a row.

```bash
echo '<table><tbody><tr><td>Edema</td><td>10.8%</td></tr><tr><td>Headache</td><td>0.6%</td></tr></tbody></table>' | htmlmd --profile gfm
# actual:
#   Edema
#
#   10.8%
#
#   Headache
#
#   0.6%
```

The table machinery works; it just does not engage without a header row:

```bash
echo '<table><thead><tr><th>Effect</th><th>Rate</th></tr></thead><tbody><tr><td>Edema</td><td>10.8%</td></tr></tbody></table>' | htmlmd --profile gfm
# actual (correct):
#   | Effect | Rate  |
#   | ------ | ----- |
#   | Edema  | 10.8% |
```

**Why this destroys content, not formatting.** Adverse-reaction incidence tables
in drug labels routinely omit `<thead>`. After conversion, "Edema" and "10.8%"
are two unrelated lines, so the association between an adverse effect and its
rate is gone.

A worked example from the levothyroxine (SYNTHROID) label. Phosphate binders
interfering with absorption is exactly the kind of fact a patient can act on, and
it lives here:

```html
<td class="Lrule Rrule Toprule">Phosphate Binders<br>&nbsp;&nbsp;&nbsp;(e.g.,
calcium carbonate, ferrous<br>&nbsp;&nbsp;&nbsp;sulfate, sevelamer, lanthanum)</td>
```

Under defect 1 that table is emitted as raw markup; under defect 2 its row
structure is lost. The consuming project added a dedicated Drug Interactions
extractor specifically to capture this fact and still could not, because there is
no recoverable sentence in the converted output.

**Suggested fix:** when no `<thead>` is present, treat the first `<tr>` as the
header, or emit a headerless table. Failing that, join each row's cells onto one
line so the association survives.

---

## 3. Definition lists orphan the term from its definition

`<dl>/<dt>/<dd>` converts to two unconnected blocks.

```bash
printf '<dl><dt>&bull;</dt><dd>Serious reactions have occurred.</dd></dl>' | htmlmd --profile plain-text
# actual:
#   •
#
#   Serious reactions have occurred.
```

DailyMed writes prescribing-information highlights as a definition list whose
`<dt>` is a literal bullet character, so the output is a `•` alone on a line,
separated from the content it belonged to.

**Corpus impact:** 3,657 orphaned bullet lines across 69 files.

This one caused a real safety-content loss downstream. Some boxed warnings are
marked up as `<dl>`, and a consumer splitting the warning into its limbs merged
the whole block into one run beginning "See full prescribing information", then
discarded it as boilerplate. A real boxed warning went missing until the consumer
special-cased the orphaned glyph.

Real `<ul>/<li>` lists convert correctly, including the
`<li><span class="Bold">...</span></li>` shape DailyMed uses elsewhere. Only
`<dl>` is affected.

**Suggested fix:** render `<dt>`/`<dd>` as a single unit, or as a list item when
the `<dt>` is purely a bullet glyph.

---

## 4. `--profile pandoc` passes block elements through as raw HTML

```bash
echo '<ul class="Disc"><li>First item.</li><li>Second item.</li></ul>' | htmlmd --profile pandoc
# actual:   <ul class="Disc"><li>First item.</li><li>Second item.</li></ul>
# expected: - First item.
#           - Second item.
```

The same input converts correctly under `plain-text`, `commonmark`, `gfm` and
`extended`. Tables behave the same way. If raw-HTML passthrough is deliberate for
this profile it should be documented in `docs/PROFILES.md`; if not, it is defect
1's failure mode reached by a different route.

---

## 5. U+00A0 is preserved in headings, including as the number-to-title separator

Non-breaking spaces survive conversion. Two shapes:

- inside heading text: `## 1.1<nbsp> Skin and Skin Structure<nbsp> Infections`
- as the separator between a section number and its title, where DailyMed emits a
  run of them: `1<nbsp><nbsp><nbsp><nbsp><nbsp> INDICATIONS AND USAGE`, verified
  with `od -c` as `1 302 240 302 240 ... I N D I C A T I O N S`

**Corpus impact:** 60 files.

Preserving U+00A0 is defensible as fidelity. The problem is that it is
undetectable to a consumer writing an ordinary heading pattern, and which
consumers break depends on their regex engine: JavaScript's `\s` matches U+00A0,
so a JS consumer is unaffected, while `mawk` and byte-oriented matchers are not.
In this corpus a section-extraction pattern anchored on `[0-9]+ +[A-Z]` silently
never matched, and a capture ran to end of file instead of stopping at the next
section, producing 100 KB "sections".

**Suggested fix:** a `--normalise-whitespace` flag folding U+00A0, U+2007 and
U+202F to U+0020. Preserving them by default is a fair choice; offering no way to
turn it off is not.

---

## 6. `<img alt>` text is emitted as body prose

Alt attribute content is rendered into the document body as though it were a
sentence the page contains.

```bash
echo '<p>Real sentence.</p><img alt="Alt text that reads like a sentence and contains 200 mg.">' | htmlmd --profile plain-text
# actual: both strings appear as body text, indistinguishable from each other
```

Found on the ribociclib (KISQALI) label, where the alt attribute reads:

> The Following chemical structure of KISQALI film-coated tablets are supplied
> for oral administration and contain 200 mg of ribociclib free base (equivalent
> to 254.40 mg ribociclib succinate)

That is an accessibility description that also garbles a structure caption
together with HOW SUPPLIED content. It reads exactly like a real label sentence
and carries a plausible number.

**Why this is worse than noise.** It is a fabrication vector. A consumer quoting
from the converted file treats it as fact, and any checker comparing a quotation
against that same converted file cannot detect the problem, because the string
genuinely is in the file. Only re-fetching the live page and comparing against
rendered text catches it, which is how it was found.

Rare in this corpus (1 label in 471 with this signature) but silent.

**Suggested fix:** do not inline `alt` text into body prose by default, or
annotate it so consumers can filter it.

---

## Not defects, recorded to save the next investigator the time

Two behaviours looked like htmlmd bugs and are not.

- **Expanded cross-reference anchors.** A DailyMed boxed warning converts with
  `[see Warnings and Precautions (5.1)]` where the rendered page appears to show
  only `(5.1)`. The source HTML genuinely contains
  `[see <a href="#s5.1">Warnings and Precautions (5.1)</a>]` in the full
  prescribing information; the short form is a *different element* in the
  highlights block. htmlmd is faithful to both.

- **HTML entities such as `&#8805;`.** htmlmd decodes these correctly. The
  mismatch observed was in the consumer's own comparison code, which decoded only
  a handful of named entities.

---

## Regression fixtures

Each repro above is a one-line stdin case suitable for a table-driven test, for
example:

```rust
#[test]
fn table_with_span_attribute_is_not_passed_through_raw() {
    let html = r#"<table><tbody><tr><td colspan="">Cell text.</td></tr></tbody></table>"#;
    let out = convert(html, Profile::PlainText);
    assert!(!out.contains("<table"), "raw markup leaked: {out}");
    assert!(out.contains("Cell text."));
}
```

The reporting project can supply the original HTML for any case on request; the
labels are public at `dailymed.nlm.nih.gov` and `medicines.org.uk/emc`.
