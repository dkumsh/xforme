//! PDF renderer, backed by `printpdf` (the spiritual successor to the iText
//! output of the original Java project).
//!
//! Layout is a straightforward top-to-bottom flow of fixed-height rows on a
//! single A4 page. Cell text is positioned within its column span according to
//! the cell's alignment; fills and top borders are drawn beneath the text.

use crate::document::{Cell, Document, Row};
use crate::template::{Align, NumberFormat};
use crate::value::{Value, format_currency};
use printpdf::*;
use std::path::Path;

const PAGE_W_MM: f32 = 210.0;
const PAGE_H_MM: f32 = 297.0;
const MARGIN_MM: f32 = 20.0;
const CELL_PAD_MM: f32 = 1.5;
const DEFAULT_FONT_PT: f32 = 10.0;
/// Points -> millimetres.
const PT_TO_MM: f32 = 0.352_777_8;

/// Render `doc` to a `.pdf` file at `path`.
pub fn render_pdf(doc: &Document, path: impl AsRef<Path>) -> std::io::Result<()> {
    let mut document = PdfDocument::new(&doc.title);

    // Left edge of each column, in mm from the page's left edge.
    let mut col_x = Vec::with_capacity(doc.columns.len() + 1);
    let mut x = MARGIN_MM;
    col_x.push(x);
    for col in &doc.columns {
        x += col.pdf_width as f32;
        col_x.push(x);
    }

    let mut ops: Vec<Op> = Vec::new();
    let mut y_top = PAGE_H_MM - MARGIN_MM;

    for row in &doc.rows {
        let font_pt = row_font_size(row);
        let row_h = font_pt * 1.2 * PT_TO_MM + 2.0;
        let y_bottom = y_top - row_h;
        let baseline = y_bottom + CELL_PAD_MM;

        emit_row(&mut ops, row, &col_x, y_top, y_bottom, baseline, font_pt);
        y_top = y_bottom;
    }

    let page = PdfPage::new(Mm(PAGE_W_MM), Mm(PAGE_H_MM), ops);
    let bytes = document
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut Vec::new());
    std::fs::write(path, bytes)
}

fn row_font_size(row: &Row) -> f32 {
    row.cells
        .iter()
        .filter_map(|c| c.style.font_size)
        .fold(DEFAULT_FONT_PT, f32::max)
}

fn emit_row(
    ops: &mut Vec<Op>,
    row: &Row,
    col_x: &[f32],
    y_top: f32,
    y_bottom: f32,
    baseline: f32,
    font_pt: f32,
) {
    let mut col_idx: usize = 0;
    for cell in &row.cells {
        let start = col_idx.min(col_x.len() - 1);
        let end = (col_idx + cell.colspan as usize).min(col_x.len() - 1);
        let x_start = col_x[start];
        let x_end = col_x[end];

        // Background fill, drawn first so text sits on top.
        if let Some(rgb) = cell.style.fill {
            push_fill_rect(ops, x_start, x_end, y_bottom, y_top, rgb);
        }
        // Top rule for total separators.
        if cell.style.border_top {
            push_hline(ops, x_start, x_end, y_top);
        }

        let text = cell_text(cell);
        if !text.is_empty() {
            let size = cell.style.font_size.unwrap_or(font_pt);
            let bold = cell.style.bold;
            let text_w = estimate_width_mm(&text, size, bold);
            let tx = match cell.style.align {
                Align::Left => x_start + CELL_PAD_MM,
                Align::Right => x_end - CELL_PAD_MM - text_w,
                Align::Center => x_start + (x_end - x_start - text_w) / 2.0,
            };
            push_text(ops, &text, tx, baseline, size, bold);
        }

        col_idx = end;
    }
}

/// The display string for a cell, honouring its number format.
fn cell_text(cell: &Cell) -> String {
    match &cell.value {
        Value::Empty => String::new(),
        Value::Text(s) => s.clone(),
        Value::Number(n) => match cell.style.number_format {
            NumberFormat::Currency => format_currency(*n),
            NumberFormat::Plain => crate::value::format_number(*n),
        },
    }
}

fn builtin(bold: bool) -> BuiltinFont {
    if bold {
        BuiltinFont::HelveticaBold
    } else {
        BuiltinFont::Helvetica
    }
}

fn black() -> Color {
    Color::Rgb(Rgb {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        icc_profile: None,
    })
}

fn rgb_from_u32(rgb: u32) -> Color {
    let r = ((rgb >> 16) & 0xFF) as f32 / 255.0;
    let g = ((rgb >> 8) & 0xFF) as f32 / 255.0;
    let b = (rgb & 0xFF) as f32 / 255.0;
    Color::Rgb(Rgb {
        r,
        g,
        b,
        icc_profile: None,
    })
}

fn push_text(ops: &mut Vec<Op>, text: &str, x_mm: f32, y_mm: f32, size_pt: f32, bold: bool) {
    ops.extend_from_slice(&[
        Op::StartTextSection,
        Op::SetTextCursor {
            pos: Point::new(Mm(x_mm), Mm(y_mm)),
        },
        Op::SetFont {
            font: PdfFontHandle::Builtin(builtin(bold)),
            size: Pt(size_pt),
        },
        Op::SetLineHeight { lh: Pt(size_pt) },
        Op::SetFillColor { col: black() },
        Op::ShowText {
            items: vec![TextItem::Text(text.to_string())],
        },
        Op::EndTextSection,
    ]);
}

fn push_fill_rect(ops: &mut Vec<Op>, x0: f32, x1: f32, y0: f32, y1: f32, rgb: u32) {
    ops.push(Op::SetFillColor {
        col: rgb_from_u32(rgb),
    });
    ops.push(Op::DrawPolygon {
        polygon: Polygon {
            rings: vec![PolygonRing {
                points: vec![
                    corner(x0, y0),
                    corner(x1, y0),
                    corner(x1, y1),
                    corner(x0, y1),
                ],
            }],
            mode: PaintMode::Fill,
            winding_order: WindingOrder::NonZero,
        },
    });
}

fn push_hline(ops: &mut Vec<Op>, x0: f32, x1: f32, y: f32) {
    ops.push(Op::SetOutlineColor { col: black() });
    ops.push(Op::SetOutlineThickness { pt: Pt(0.6) });
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![corner(x0, y), corner(x1, y)],
            is_closed: false,
        },
    });
}

fn corner(x_mm: f32, y_mm: f32) -> LinePoint {
    LinePoint {
        p: Point::new(Mm(x_mm), Mm(y_mm)),
        bezier: false,
    }
}

/// Rough text-width estimate for alignment. Built-in Helvetica averages a touch
/// over half the point size per glyph; bold a little wider.
fn estimate_width_mm(text: &str, size_pt: f32, bold: bool) -> f32 {
    let factor = if bold { 0.55 } else { 0.50 };
    text.chars().count() as f32 * size_pt * factor * PT_TO_MM
}
