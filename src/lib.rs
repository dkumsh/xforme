//! # xforme
//!
//! A blazing-fast Rust engine that streams your data through ordinary Excel
//! templates to mass-produce pixel-perfect spreadsheets and PDFs, formatting
//! and formulas intact.
//!
//! It is the Rust migration of *templateIt* — the 2009 Java project by the same
//! author (<https://templateit.sourceforge.net/>): a data-driven template processor
//! that fills a template from a tab-delimited data stream and renders the
//! result to both spreadsheet and PDF — the roles played by Apache POI and
//! iText in the original.
//!
//! The pipeline is:
//!
//! ```text
//! data file ──[data::parse]──▶ Sheet ─┐
//!                                      ├─[engine::process]──▶ Document ──┬─[render_xlsx]──▶ .xlsx
//! Template ────────────────────────────┘                                └─[render_pdf]───▶ .pdf
//! ```
//!
//! The bundled [`salesreceipt`] module reproduces the project's canonical Sales
//! Receipt example on top of the generic engine.

pub mod data;
pub mod demo_template;
pub mod document;
pub mod engine;
pub mod expr;
pub mod render;
pub mod salesreceipt;
pub mod template;
pub mod value;
pub mod xlsx_template;

pub use document::Document;
pub use engine::process;
pub use render::{render_pdf, render_xlsx};
pub use template::Template;
