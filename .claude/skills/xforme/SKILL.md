---
name: xforme
description: >-
  Build and drive xforme Excel-template reports. Use when creating or editing an
  .xlsx template (by hand or programmatically with umya-spreadsheet), wiring up
  the render/parse API to mass-produce .xlsx/PDF reports from a data stream
  (tab-delimited, JSON, YAML, CSV), or debugging why a rendered workbook is
  missing charts, images, conditional formatting, or formula results.
---

# xforme — build and drive Excel templates

xforme streams a data file through an **ordinary `.xlsx` workbook** (designed by a
human or generated in code) and emits a populated `.xlsx` (optionally a PDF). The
engine edits the template sheet **in place** — fills parameters, grows the
repeating rows, renames the sheet — so it preserves *everything* the designer put
in the workbook: styles, number formats, merges, native formulas, **conditional
formatting, in-cell data bars, images, and charts** (chart series ranges even grow
with the data).

```text
data file ──[data::parse*]──▶ Sheet ──┐
                                      ├─[xlsx_template::render*]──▶ .xlsx report ──▶ (pdf::to_pdf*)
template.xlsx ────────────────────────┘
```

The canonical worked example is `src/demo_template.rs` (builds the bundled Sales
Receipt and Portfolio Statement templates). Read it before writing a new template
generator. End-user reference is `README.md`.

---

## A. Using the crate (consumers)

Add it (features `pdf`, `json`, `yaml`, `csv` are all on by default):

```toml
[dependencies]
xforme = "0.2"
# .xlsx-only, no subprocess/serde deps:
# xforme = { version = "0.2", default-features = false }
```

Parse a data stream into `Sheet`s, then render the first one against a template:

```rust
use std::path::Path;
use xforme::{data, xlsx_template};

let sheets = data::parse(tsv_str)?;            // or parse_json / parse_yaml / parse_csv
let sheet  = &sheets[0];

// Template in: file path OR in-memory bytes (anything Into<TemplateSource>:
// &Path, &[u8], &Vec<u8> — e.g. include_bytes!, a DB blob, a network buffer).
let warnings = xlsx_template::render_to_file(Path::new("template.xlsx"), sheet, "report.xlsx")?;
for w in &warnings { eprintln!("warning: {w}"); }   // e.g. a data label matching no template row
```

| Function | Template in | Returns |
| --- | --- | --- |
| `render` | path or bytes | `Workbook` (post-process before saving) |
| `render_with_warnings` | path or bytes | `(Workbook, Vec<String>)` |
| `render_to_file` | path or bytes | `Vec<String>` warnings (writes the file) |
| `render_to_bytes` | path or bytes | `Vec<u8>` |

PDF (needs `soffice`/LibreOffice on `PATH`; `pdf` feature):

```rust
let pdf_path  = xforme::pdf::to_pdf_file("report.xlsx")?;   // file -> file
let pdf_bytes = xforme::pdf::to_pdf_bytes(&report_xlsx)?;   // bytes -> bytes (temp dir)
```

CLI: `cargo run -- --template T.xlsx --data D.{txt,json,yaml,csv} [--out PREFIX] [--no-pdf]`.

---

## B. Data stream formats

A stream is one or more sheets. Each **record** has a `label` (matches a column-A
control label in the template) plus fields.

- **Tab-delimited** (always available): `#sheet <template> <title>` opens a sheet
  (template = which sheet inside the `.xlsx` to fill; title = output tab name);
  `##end` closes it. Each line: first field = label, rest = **positional** fields
  (`#1`, `#2`, … in the template).
- **JSON / YAML / CSV** (features `json`/`yaml`/`csv`): same record stream. JSON &
  YAML records carry **named** fields, so `#name` resolves directly and the
  column-A schema becomes optional; a positional `"fields": [...]` array is also
  accepted. CSV is the positional stream, comma-separated with quoting.

```yaml
template: Portfolio          # sheet name inside the .xlsx to fill
title: Portfolio Statement   # output tab name
records:
  - { label: header, account: "0042-118827", holder: Jane Roe, period: May 2026 }
  - { label: row1, symbol: AAPL, qty: 50, cost: 9200, market: 11540 }
  - { label: footer }
```

`data::Sheet { template, title, records }`, `Record { label, fields, named }`.

---

## C. The template contract (authors — read this before designing)

A template is a **normal, openable Excel workbook**. xforme reads a few
lightweight conventions:

1. **Column A holds a control label per row** (the column is hidden on output):
   - `header` / `footer` — emitted once, from the `header` / `footer` record.
   - any other non-empty label (`row1`, `row2`, …) — a **detail** row. The
     contiguous run of detail rows is the **detail band**; one output row is
     emitted per matching record, choosing the template row whose label matches
     (alternate `row1`/`row2` for zebra striping in data order).
   - **blank** — a static row (title, column headers, spacers), emitted verbatim.
2. **Declare each band's field names once**, in the label on its first row:
   `header(date,receipt,customer,address)`. This maps `#name` params to positional
   fields. Bare `header` on later rows just attaches to it.
3. **Parameters live in cell comments**: a cell whose comment is `#name` (from the
   schema) or `#N` (1-based positional) is filled from data at render. **Leave a
   real sample value in the cell** so formulas compute and number formats preview
   while designing; xforme overwrites it and strips the comment.
4. **Formulas are native Excel.** When a row is replicated downward, xforme
   row-shifts *relative* references (`=B7*D7` → `=B12*D12`) and leaves
   `$`-anchored rows fixed. For a **total over the variable-length band**, use
   mixed anchoring — `=SUM(E$7:E8)` — so the anchored start stays put and the
   relative end grows to cover every rendered detail row.
5. Formula **results** are recomputed by Excel/LibreOffice on open — xforme stores
   the formula text and clears cached values.

Worked grid (the Sales Receipt; `«#x»` = a cell comment, values are samples):

| Row | A (label) | B | C | D | E |
|----|----|----|----|----|----|
| 2  | `header(date,receipt,customer,address)` | Receipt #: | `NNNNN` «#receipt» | Date: | `MM/DD/YYYY` «#date» |
| 7  | `row1(seq,qty,desc,price)` | `2` «#qty» | `Sample item` «#desc» | `9.99` «#price» | `=B7*D7` |
| 8  | `row2(seq,qty,desc,price)` | `3` «#qty» | … | `4.99` «#price» | `=B8*D8` |
| 10 | `footer(taxrate)` | | | Subtotal: | `=SUM(E$7:E8)` |

Render with 4 line items → the band expands to 4 rows and the `SUM` grows to
`=SUM(E$7:E10)`.

### Authoring by hand (Excel / LibreOffice)

Lay the document out as a finished sample, type labels in column A, add `#name`
comments to the data cells (keep sample values), write native formulas with mixed
anchoring for band totals, save as `.xlsx`. Done — no code.

---

## D. Authoring a template programmatically (umya-spreadsheet) — and the landmines

If you generate the template in Rust (like `src/demo_template.rs`), you build it
with `umya-spreadsheet` directly. **These umya 3.0 behaviors will silently break
output — none are in umya's docs.** Each cost real debugging; honor them:

1. **Never set page setup on a template that has images or charts.** umya reserves
   a relationship id for printer settings whenever `page_setup().has_param()` is
   true, but only emits the printer-settings relationship when a binary blob is
   present (it isn't). The off-by-one shifts the `<drawing>` r:id past its rels
   entry, and **Excel/LibreOffice silently drop the entire drawing layer** (logo +
   all charts). Don't call `set_orientation` / `set_fit_to_width` / etc. on such
   sheets.
2. **Always set an explicit font color on bold/styled cells.** A fresh
   `Style::default()` whose font you touch (e.g. `.font_mut().set_bold(true)`)
   serializes its color as `indexed="1"` = **white** → invisible text on white.
   Set a real color (`font.color_mut().set_argb_str("FF1F3A5F")`) on every styled
   cell.
3. **`cellIs` conditional-format rules need a `<formula>` child**, not text:
   ```rust
   let mut f = umya_spreadsheet::Formula::default();
   f.set_string_value("0");                 // the comparison value
   rule.set_type(ConditionalFormatValues::CellIs);
   rule.set_operator(ConditionalFormattingOperatorValues::LessThan);
   rule.set_formula(f);                     // NOT set_text("0")
   ```
4. **Data bars** = a `DataBar` with two CFVOs (`Min`, `Max`) + one color, attached
   via `rule.set_data_bar(bar)` with `set_type(ConditionalFormatValues::DataBar)`.
5. **Charts** built with `chart.new_chart(&ChartType::BarChart, from, to, vec![range])`
   treat **every** vector entry as a *value* series (no categories). To label
   bars, attach `CategoryAxisData` (a `StringReference` whose `Formula` address is
   the label range) to each series via `series.set_category_axis_data(...)`.
   Series ranges must be sheet-qualified: `"Portfolio!$E$8:$E$9"`.
6. **Chart series + sheet rename:** umya grows a series' *row* range on row insert,
   but does **not** retarget the *sheet name* on rename — leaving `OldSheet!...`,
   which fails to serialize. The xforme engine already fixes this in
   `fill_in_place` (`retarget_chart_series`); if you build similar tooling, you
   must rewrite each series `Formula`'s `address_mut().set_sheet_name(new)`.
7. Import paths that aren't at the crate root:
   `umya_spreadsheet::structs::drawing::spreadsheet::MarkerType`,
   `umya_spreadsheet::structs::drawing::charts::{CategoryAxisData, StringReference, Formula}`.

### Verify visually (the only reliable check)

XML inspection isn't enough — render and look. The repo has LibreOffice + PIL:

```sh
cargo run --bin make_sample_template          # regenerate templates/*.xlsx
cargo run -- --template templates/X.xlsx --data data/X.yaml --out /tmp/out --no-pdf
soffice --headless --convert-to pdf --outdir /tmp /tmp/out.xlsx
pdftoppm -png -r 130 /tmp/out.pdf /tmp/shot   # then open the PNG and eyeball every feature
```

Also grep the produced `.xlsx` (it's a zip) to confirm structure survived:
`unzip -o out.xlsx -d x && grep -o '<c:f>[^<]*</c:f>' x/xl/charts/*.xml` (chart
ranges grew + sheet retargeted), and check `x/xl/worksheets/_rels/sheet1.xml.rels`
matches the `<drawing r:id>` in `sheet1.xml` (mismatch = dropped drawings, gotcha 1).

---

## E. Files to know

- `src/demo_template.rs` — canonical template builders (copy its patterns).
- `src/xlsx_template.rs` — the engine: contract docs, in-place fill, formula
  shifting, chart retargeting.
- `src/data.rs` — the stream parsers and `Sheet`/`Record` shapes.
- `tests/xlsx_template_e2e.rs` — invariants (band growth, CF/chart/image survival).
- `README.md` — end-user walkthrough and the Portfolio Statement showcase.
