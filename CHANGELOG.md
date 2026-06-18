# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
