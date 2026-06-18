//! XLSX renderer, backed by `rust_xlsxwriter` (the spiritual successor to the
//! Apache POI output of the original Java project).

use crate::document::{Cell, Document};
use crate::template::{Align, NumberFormat};
use crate::value::Value;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, XlsxError};
use std::path::Path;

/// Render `doc` to an `.xlsx` file at `path`.
pub fn render_xlsx(doc: &Document, path: impl AsRef<Path>) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    let sheet_name = sanitize_sheet_name(&doc.title);
    sheet.set_name(&sheet_name)?;

    for (i, col) in doc.columns.iter().enumerate() {
        sheet.set_column_width(i as u16, col.xlsx_width)?;
    }

    for (r, row) in doc.rows.iter().enumerate() {
        let row_idx = r as u32;
        let mut col_idx: u16 = 0;
        for cell in &row.cells {
            let format = build_format(cell);
            let last_col = col_idx + cell.colspan - 1;

            if cell.colspan > 1 {
                // merge_range needs a value up front; write the merged content here.
                write_merged(sheet, row_idx, col_idx, last_col, cell, &format)?;
            } else {
                write_cell(sheet, row_idx, col_idx, cell, &format)?;
            }
            col_idx = last_col + 1;
        }
    }

    workbook.save(path.as_ref())?;
    Ok(())
}

fn write_cell(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    cell: &Cell,
    format: &Format,
) -> Result<(), XlsxError> {
    match &cell.value {
        Value::Number(n) => sheet.write_number_with_format(row, col, *n, format)?,
        Value::Text(s) => sheet.write_string_with_format(row, col, s, format)?,
        Value::Empty => sheet.write_blank(row, col, format)?,
    };
    Ok(())
}

fn write_merged(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    first_col: u16,
    last_col: u16,
    cell: &Cell,
    format: &Format,
) -> Result<(), XlsxError> {
    // `merge_range` only accepts a string, so write the merged value as text and
    // then overwrite the anchor cell with a typed value to preserve numbers.
    sheet.merge_range(row, first_col, row, last_col, "", format)?;
    write_cell(sheet, row, first_col, cell, format)?;
    Ok(())
}

fn build_format(cell: &Cell) -> Format {
    let mut format = Format::new();
    let style = &cell.style;

    if style.bold {
        format = format.set_bold();
    }
    format = match style.align {
        Align::Left => format.set_align(FormatAlign::Left),
        Align::Center => format.set_align(FormatAlign::Center),
        Align::Right => format.set_align(FormatAlign::Right),
    };
    if let NumberFormat::Currency = style.number_format {
        format = format.set_num_format("$#,##0.00");
    }
    if let Some(rgb) = style.fill {
        format = format.set_background_color(Color::RGB(rgb));
    }
    if style.border_top {
        format = format.set_border_top(FormatBorder::Thin);
    }
    if let Some(size) = style.font_size {
        format = format.set_font_size(size as f64);
    }
    format
}

/// Excel sheet names are limited to 31 chars and forbid `[]:*?/\`.
fn sanitize_sheet_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| if "[]:*?/\\".contains(c) { ' ' } else { c })
        .collect();
    let trimmed = cleaned.trim();
    let name = if trimmed.is_empty() {
        "Sheet1"
    } else {
        trimmed
    };
    name.chars().take(31).collect()
}
