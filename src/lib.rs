//! # xforme
//!
//! A blazing-fast Rust engine that streams your data through ordinary Excel
//! templates to mass-produce pixel-perfect spreadsheets, formatting and
//! formulas intact.
//!
//! It is the Rust migration of *templateIt* — the 2009 Java project by the same
//! author (<https://templateit.sourceforge.net/>): a data-driven template
//! processor that fills a designer-authored `.xlsx` template from a
//! tab-delimited data stream — the role played by Apache POI in the original.
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
//! real sample values, and native Excel formulas). PDF, when needed, is just a
//! downstream conversion of the produced `.xlsx` (e.g. via LibreOffice).
//!
//! [`demo_template`] builds the bundled sample Sales Receipt template.
//!
//! With the default `pdf` feature, [`pdf`] converts a produced `.xlsx` to PDF
//! via a headless LibreOffice.

pub mod data;
pub mod demo_template;
pub mod xlsx_template;

#[cfg(feature = "pdf")]
pub mod pdf;
