# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-06-20

Focused the project on the Excel-template workflow: fill a designer-authored
`.xlsx` and produce `.xlsx` (with optional PDF). Removed the parallel
code-defined ("declarative") engine.

### Removed

- The declarative engine and its modules (`engine`, `template`, `expr`, `value`,
  `document`, `render`, `salesreceipt`) — code-defined templates rendered to
  XLSX/PDF in pure Rust. It duplicated the Excel-template engine for `.xlsx`
  output and required maintaining a separate hand-written PDF renderer.
- Dependencies **`printpdf`** and **`rust_xlsxwriter`** (only used by the above).

### Added

- **`pdf` module** (Cargo feature `pdf`, on by default): optional PDF output by
  converting a produced `.xlsx` with a headless **LibreOffice** —
  `pdf::to_pdf_file` (file → file) and `pdf::to_pdf_bytes` (in-memory, via a
  temp dir). Building with `default-features = false` yields an `.xlsx`-only
  crate that cannot spawn a subprocess.
- **JSON, YAML, and CSV data inputs** (`data::parse_json` / `parse_yaml` /
  `parse_csv`, Cargo features `json` / `yaml` / `csv`, all on by default; YAML
  via `serde_norway`, CSV via `csv`). JSON/YAML use the same record-stream model
  but records carry **named** fields resolved directly by `#name` (the column-A
  schema becomes optional; a positional `fields` array is also accepted). CSV is
  the record stream comma-separated with proper quoting. The CLI selects the
  parser by file extension (`.json` / `.yaml` / `.yml` / `.csv`, else
  tab-delimited).
- **Non-fatal warnings** via `render_with_warnings`: a data record whose label
  matches no template row is reported (the CLI prints them to stderr) instead of
  being silently ignored. `render_to_file` now returns the warnings.
- **Portfolio Statement showcase demo**
  (`demo_template::sample_portfolio_statement_template_bytes`,
  `examples/portfolio_statement.rs`, `data/portfolio_statement.{txt,json,yaml,csv}`,
  committed template `templates/portfolio_statement_template.xlsx`). It exercises
  the full feature set the engine preserves and grows: an embedded **logo image**,
  **conditional formatting** (green/red on the gain column), **in-cell data bars**
  (a market-value histogram), grown mixed-anchor `SUM` totals, and two **column
  charts** whose series ranges stretch with the holdings. The README now leads
  with it. Caveat baked into the template: it sets **no page setup**, because
  umya 3.0 reserves a relationship id for printer settings whenever page setup
  has any parameter but only emits that relationship with a binary blob present —
  the resulting r:id shift makes Excel/LibreOffice drop the whole drawing layer
  (logo + charts). Avoid page setup on templates that contain images or charts.

### Changed

- The `.xlsx` is generated entirely in memory (`umya-spreadsheet`); PDF is a
  downstream conversion of it (Excel-faithful, since LibreOffice recalculates
  the formulas). The CLI emits both by default; `--no-pdf` skips the PDF.
- **Render by editing the template sheet in place** instead of building a fresh
  sheet. The engine resizes the detail band with row insert/remove (which grows
  spanning ranges like a footer `SUM` or a conditional-format range), fills
  params, and renames the sheet. This **preserves everything in the workbook** —
  conditional formatting, images, charts, data validations, print setup, frozen
  panes, column widths — not just cell values/styles. Cached formula results are
  cleared so Excel/LibreOffice recompute on open. Designer-set row heights are
  carried over.
- The engine now reads `#name` parameter markers from **rich-text comments** too
  (the form Excel/LibreOffice write when a template is edited and saved), so a
  re-saved template keeps binding its parameters.
- **Charts now follow the rename and grow with the detail band.** umya already
  expands a chart series' row range when rows are inserted (e.g. `$E$8:$E$9` →
  `$E$8:$E$14`); the engine additionally **retargets each chart series' sheet
  reference** to the renamed output sheet. Without that, a chart authored over
  the band pointed at the (now gone) template sheet and the workbook failed to
  serialize.

## [0.1.1] - 2026-06-18

Reworked Excel-template parameters so a template is a fully valid, designable
Excel workbook.

### Changed

- **Parameters are now bound via cell comments** (`#name`, or `#N` positional)
  while the cell keeps a **real sample value**. Previously `${n}` was written
  into the cell value, which produced `#VALUE!` in formulas that referenced it
  and prevented number-format preview. Now formulas compute and formats preview
  while designing, and the template opens cleanly in Excel and POI-based viewers.
- **Field-name schema declared once per label in column A**, e.g.
  `header(date,receipt,customer,address)`, mapping `#name` parameters to
  positional data fields.
- **Totals over the detail band use native Excel mixed anchoring**
  (`=SUM(E$7:E8)`) instead of the `${firstrow}`/`${lastrow}` markers, which are
  removed. The engine expands the anchored range as the band grows.

### Fixed

- Templates no longer trigger `FormulaParseException` / `#VALUE!` in Excel
  formula evaluators (e.g. the ExcelReader IDE plugin), because formulas and
  parameter cells are now valid Excel.

## [0.1.0] - 2026-06-18

Initial release. The Rust migration of *templateIt* — the 2009 Java project by
the same author: stream a tab-delimited data file through an Excel template to
produce spreadsheets and PDFs with formatting and formulas intact.

### Added

- **Excel-template engine** (`xlsx_template`): reads a designer-authored `.xlsx`
  template, fills a new tab from the data stream, removes the template tab, and
  preserves styles, number formats, merges and native formulas (via
  `umya-spreadsheet`).
  - Control tags in column A (`header` / `footer` / detail rows such as
    `row1`/`row2`), placeholders `${n}`, and `${firstrow}`/`${lastrow}`/`${row}`
    markers.
  - Relative-reference **row-shifting** so designer formulas stay correct when a
    template row is replicated (`=B7*D7` → `=B12*D12`); absolute refs untouched.
  - Template input as a **file path or in-memory byte array**
    (`TemplateSource`); output as a file (`render_to_file`), bytes
    (`render_to_bytes`), or a `Workbook` (`render`).
- **Declarative engine** (`engine` + `template`): code-defined templates with a
  small arithmetic/aggregate expression evaluator, rendered to XLSX
  (`rust_xlsxwriter`) and PDF (`printpdf`, built-in Helvetica).
- **Data-stream parser** (`data`) for the templateIt format
  (`#sheet` … records … `##end`).
- **Generic `xforme` binary**: `--template TEMPLATE.xlsx --data DATA.txt
  [--out PREFIX] [--no-pdf]`; converts the rendered spreadsheet to PDF via a
  headless LibreOffice when available.
- **Self-contained example** (`examples/sales_receipt.rs`) embedding the
  template and data with `include_bytes!` / `include_str!`.
- **`make_sample_template` binary** to (re)generate the committed sample
  template asset.
