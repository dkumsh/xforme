//! # xforme
//!
//! A blazing-fast Rust engine that streams your data through ordinary Excel
//! templates to mass-produce richly-formatted `.xlsx` spreadsheets — formatting
//! and formulas intact.
//!
//! It is the Rust migration of *templateIt* — the 2009 Java project by the same
//! author (<https://templateit.sourceforge.net/>): a data-driven template
//! processor that fills a designer-authored `.xlsx` template from a data stream
//! — the role played by Apache POI in the original.
//!
//! The pipeline is:
//!
//! ```text
//! data file ──[data::parse]──▶ Sheet ─┐
//!                                      ├─[xlsx_template::render]──▶ .xlsx report
//! template.xlsx ───────────────────────┘
//! ```
//!
//! The template is a real Excel workbook: see [`xlsx_template`] for the
//! convention (column-A control labels, comment-bound `#name` parameters with
//! real sample values, and native Excel formulas). The engine edits the template
//! sheet in place, so it preserves *everything* the designer put in the
//! workbook — styles, number formats, merges, **conditional formatting, in-cell
//! data bars, images, and charts** (whose series ranges even grow with the data)
//! — not just cell values.
//!
//! Data can be [tab-delimited](data::parse) or, behind the default-on `json` /
//! `yaml` / `csv` features, [JSON](data::parse_json), [YAML](data::parse_yaml),
//! or [CSV](data::parse_csv). PDF, when needed, is just a downstream conversion
//! of the produced `.xlsx`: with the default `pdf` feature, [`pdf`] converts one
//! via a headless LibreOffice.
//!
//! [`demo_template`] builds the two bundled sample templates — a simple Sales
//! Receipt and the Portfolio Statement showcase (logo, conditional formatting,
//! data bars, and charts).

pub mod data;
pub mod demo_template;
pub mod xlsx_template;

#[cfg(feature = "pdf")]
pub mod pdf;
