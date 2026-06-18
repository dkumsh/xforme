//! The rendered intermediate document.
//!
//! The engine turns a [`Template`](crate::template::Template) plus a parsed
//! [`Sheet`](crate::data::Sheet) into a [`Document`]: a fully-resolved grid of
//! cells with concrete values and styles. Renderers consume this and know
//! nothing about templates, expressions, or the data format.

use crate::template::{ColumnSpec, Style};
use crate::value::Value;

/// A resolved cell ready to render.
#[derive(Clone, Debug)]
pub struct Cell {
    pub value: Value,
    pub style: Style,
    pub colspan: u16,
}

/// A resolved row.
#[derive(Clone, Debug)]
pub struct Row {
    pub cells: Vec<Cell>,
}

/// A resolved document: one output sheet.
#[derive(Clone, Debug)]
pub struct Document {
    pub title: String,
    pub columns: Vec<ColumnSpec>,
    pub rows: Vec<Row>,
}

impl Document {
    /// Number of output columns.
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }
}
