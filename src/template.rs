//! The declarative template model.
//!
//! A [`Template`] is templateIt's analogue of an Excel template workbook: it
//! describes the columns of the output sheet and, for each *record label* that
//! can appear in the data stream (`header`, `row1`, `footer`, ...), a [`Band`]
//! of output rows to emit. Bands carry placeholders and expressions that the
//! engine resolves against each record's fields plus running accumulators.

use std::collections::HashMap;

/// Horizontal alignment of a cell's contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// How a numeric cell should be formatted on output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NumberFormat {
    /// Plain number, no decoration (e.g. a quantity).
    #[default]
    Plain,
    /// US currency, two decimals, thousands separators.
    Currency,
}

/// Visual styling for a cell. Honoured by every renderer as faithfully as its
/// backend allows.
#[derive(Clone, Debug, Default)]
pub struct Style {
    pub bold: bool,
    pub align: Align,
    pub number_format: NumberFormat,
    /// Optional background fill as an `0xRRGGBB` value.
    pub fill: Option<u32>,
    /// Draw a rule along the top edge of the cell (used for total separators).
    pub border_top: bool,
    /// Point size override; `None` uses the renderer's default body size.
    pub font_size: Option<f32>,
}

impl Style {
    pub fn new() -> Self {
        Style::default()
    }
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;
        self
    }
    pub fn currency(mut self) -> Self {
        self.number_format = NumberFormat::Currency;
        self
    }
    pub fn fill(mut self, rgb: u32) -> Self {
        self.fill = Some(rgb);
        self
    }
    pub fn border_top(mut self) -> Self {
        self.border_top = true;
        self
    }
    pub fn size(mut self, pt: f32) -> Self {
        self.font_size = Some(pt);
        self
    }
}

/// What produces a cell's value.
#[derive(Clone, Debug)]
pub enum Content {
    /// Static text, emitted verbatim.
    Literal(String),
    /// Text with `${...}` placeholders interpolated against the scope.
    Text(String),
    /// An arithmetic expression evaluated to a number.
    Expr(String),
    /// An empty spacer cell.
    Empty,
}

/// A single cell in a band row.
#[derive(Clone, Debug)]
pub struct CellSpec {
    pub content: Content,
    pub style: Style,
    /// Number of output columns this cell spans (>= 1).
    pub colspan: u16,
}

impl CellSpec {
    pub fn new(content: Content) -> Self {
        CellSpec {
            content,
            style: Style::new(),
            colspan: 1,
        }
    }
    pub fn literal(s: impl Into<String>) -> Self {
        CellSpec::new(Content::Literal(s.into()))
    }
    pub fn text(s: impl Into<String>) -> Self {
        CellSpec::new(Content::Text(s.into()))
    }
    pub fn expr(s: impl Into<String>) -> Self {
        CellSpec::new(Content::Expr(s.into()))
    }
    pub fn empty() -> Self {
        CellSpec::new(Content::Empty)
    }
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    pub fn span(mut self, n: u16) -> Self {
        self.colspan = n.max(1);
        self
    }
}

/// One physical output row within a band.
#[derive(Clone, Debug, Default)]
pub struct RowSpec {
    pub cells: Vec<CellSpec>,
    /// Mark alternating data rows so renderers can apply banding.
    pub banded: bool,
}

impl RowSpec {
    pub fn new(cells: Vec<CellSpec>) -> Self {
        RowSpec {
            cells,
            banded: false,
        }
    }
    pub fn banded(mut self) -> Self {
        self.banded = true;
        self
    }
}

/// The set of rows emitted for one record label, plus any accumulation it
/// performs (e.g. adding `qty * price` into `subtotal`).
#[derive(Clone, Debug, Default)]
pub struct Band {
    pub rows: Vec<RowSpec>,
    /// `(accumulator name, expression)` pairs evaluated after the rows are
    /// emitted and added into the engine's running totals.
    pub accumulate: Vec<(String, String)>,
}

/// A column of the output sheet.
#[derive(Clone, Debug)]
pub struct ColumnSpec {
    /// Width in characters, for spreadsheet output.
    pub xlsx_width: f64,
    /// Width in millimetres, for PDF layout.
    pub pdf_width: f64,
}

/// A complete template: columns + per-label field names + per-label bands.
#[derive(Clone, Debug, Default)]
pub struct Template {
    pub name: String,
    pub columns: Vec<ColumnSpec>,
    /// Field names for each record label, positional to the data file columns.
    pub fields: HashMap<String, Vec<String>>,
    /// Output bands keyed by record label.
    pub bands: HashMap<String, Band>,
}
