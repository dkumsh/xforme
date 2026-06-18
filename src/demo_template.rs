// Builds the bundled sample Sales Receipt template workbook.
//
// This stands in for the workbook a *designer* would normally craft by hand in
// Excel or LibreOffice. It exercises everything the Excel-template engine
// preserves: column-A control labels, fonts/bold, alignment, fills, number
// formats, merged cells, and native Excel formulas (line totals, a SUM over the
// variable-length detail band, tax and grand total).
//
// Parameters are bound through cell *comments* (`#name`); the cells hold real
// sample values, so the template itself renders as a working sample receipt —
// formulas compute and number formats preview. Column A declares each label's
// field-name schema once, e.g. `header(date,receipt,customer,address)`.
//
// It is exposed as a library helper and used by the `make_sample_template`
// binary to (re)generate the committed `templates/sales_receipt_template.xlsx`
// asset, which the demo binary then embeds with `include_bytes!`.

use umya_spreadsheet::{Comment, HorizontalAlignmentValues, Style};

const SHADE: &str = "FFF2F2F2"; // banded row fill (ARGB)
const HEAD_FILL: &str = "FFD9E1F2"; // column-header fill
const CURRENCY: &str = "$#,##0.00";

/// Build the sample Sales Receipt template and return it as `.xlsx` bytes.
pub fn sample_sales_receipt_template_bytes() -> Vec<u8> {
    let mut book = umya_spreadsheet::new_file();
    book.set_sheet_name(0, "SalesReceipt")
        .expect("rename sheet");
    let ws = book.sheet_by_name_mut("SalesReceipt").expect("sheet");

    // Column widths: A is the control-label column. It's hidden in the output,
    // so it's made wide enough here to read the labels/schemas while designing.
    ws.column_dimension_mut("A").set_width(16.0);
    ws.column_dimension_mut("B").set_width(8.0);
    ws.column_dimension_mut("C").set_width(40.0);
    ws.column_dimension_mut("D").set_width(13.0);
    ws.column_dimension_mut("E").set_width(13.0);

    // Row 1 — title, merged across the printed columns.
    set(
        ws,
        "B1",
        "SALES RECEIPT",
        sb().bold().size(16.0).center().done(),
    );
    ws.add_merge_cells("B1:E1");

    // Rows 2-4 — header band. Column A declares the field schema once. The cells
    // hold obvious *placeholder* samples so it's clear they're filled from data.
    set(ws, "A2", "header(date,receipt,customer,address)", plain());
    set(ws, "B2", "Receipt #:", sb().bold().done());
    param(ws, "C2", "NNNNN", "receipt", plain());
    set(ws, "D2", "Date:", sb().bold().right().done());
    param(ws, "E2", "MM/DD/YYYY", "date", sb().right().done());

    set(ws, "A3", "header", plain());
    set(ws, "B3", "Sold To:", sb().bold().done());
    param(ws, "C3", "Customer Name", "customer", plain());
    ws.add_merge_cells("C3:E3");

    set(ws, "A4", "header", plain());
    param(ws, "C4", "Street, City, State ZIP", "address", plain());
    ws.add_merge_cells("C4:E4");

    // Row 5 — spacer.

    // Row 6 — column headers (static).
    set(ws, "B6", "Qty", sb().bold().right().fill(HEAD_FILL).done());
    set(ws, "C6", "Description", sb().bold().fill(HEAD_FILL).done());
    set(
        ws,
        "D6",
        "Unit Price",
        sb().bold().right().fill(HEAD_FILL).done(),
    );
    set(
        ws,
        "E6",
        "Amount",
        sb().bold().right().fill(HEAD_FILL).done(),
    );

    // Rows 7-8 — detail band: row1 (plain) and row2 (shaded). Cells hold sample
    // values; Amount is a native Excel formula over them.
    set(ws, "A7", "row1(seq,qty,desc,price)", plain());
    param(ws, "B7", "2", "qty", sb().right().done());
    param(ws, "C7", "Sample item", "desc", plain());
    param(ws, "D7", "9.99", "price", sb().right().fmt(CURRENCY).done());
    setf(ws, "E7", "B7*D7", sb().right().fmt(CURRENCY).done());

    set(ws, "A8", "row2(seq,qty,desc,price)", plain());
    param(ws, "B8", "3", "qty", sb().right().fill(SHADE).done());
    param(
        ws,
        "C8",
        "Another sample item",
        "desc",
        sb().fill(SHADE).done(),
    );
    param(
        ws,
        "D8",
        "4.99",
        "price",
        sb().right().fill(SHADE).fmt(CURRENCY).done(),
    );
    setf(
        ws,
        "E8",
        "B8*D8",
        sb().right().fill(SHADE).fmt(CURRENCY).done(),
    );

    // Row 9 — spacer.

    // Rows 10-12 — footer band. The subtotal sums the detail band using Excel
    // mixed anchoring: the start row `E$7` is anchored to the first detail row,
    // the end row `E8` is relative to the last detail template row. As the band
    // expands and the footer slides down, the relative end grows to cover every
    // rendered detail row. Tax and total reference the cells above.
    set(ws, "A10", "footer(taxrate)", plain());
    set(ws, "D10", "Subtotal:", sb().bold().right().done());
    setf(ws, "E10", "SUM(E$7:E8)", sb().right().fmt(CURRENCY).done());

    set(ws, "A11", "footer", plain());
    set(ws, "C11", "Sales Tax", sb().right().done());
    param(
        ws,
        "D11",
        "0.1",
        "taxrate",
        sb().right().fmt("0.00%").done(),
    );
    setf(ws, "E11", "E10*D11", sb().right().fmt(CURRENCY).done());

    set(ws, "A12", "footer", plain());
    set(ws, "D12", "TOTAL:", sb().bold().right().done());
    setf(
        ws,
        "E12",
        "E10+E11",
        sb().bold().right().fmt(CURRENCY).done(),
    );

    let mut buf = Vec::new();
    umya_spreadsheet::writer::xlsx::write_writer(&book, &mut buf).expect("serialize template");
    buf
}

/// Set a static cell's literal value and style.
fn set(ws: &mut umya_spreadsheet::Worksheet, at: &str, value: &str, style: Style) {
    let cell = ws.cell_mut(at);
    cell.set_value(value);
    cell.set_style(style);
}

/// Set a cell's formula (no leading `=`) and style.
fn setf(ws: &mut umya_spreadsheet::Worksheet, at: &str, formula: &str, style: Style) {
    let cell = ws.cell_mut(at);
    cell.set_formula(formula);
    cell.set_style(style);
}

/// Set a parameter cell: a real sample `value` (so formulas/formats work) plus
/// a `#name` comment that binds it to a data field at render time.
fn param(ws: &mut umya_spreadsheet::Worksheet, at: &str, value: &str, name: &str, style: Style) {
    {
        let cell = ws.cell_mut(at);
        cell.set_value(value);
        cell.set_style(style);
    }
    let mut comment = Comment::default();
    comment.new_comment(at);
    comment.set_text_string(format!("#{name}"));
    ws.add_comments(comment);
}

fn plain() -> Style {
    Style::default()
}

fn sb() -> StyleBuilder {
    StyleBuilder {
        st: Style::default(),
    }
}

/// Small fluent builder over umya's `Style`.
struct StyleBuilder {
    st: Style,
}

impl StyleBuilder {
    fn bold(mut self) -> Self {
        self.st.font_mut().set_bold(true);
        self
    }
    fn size(mut self, pt: f64) -> Self {
        self.st.font_mut().set_size(pt);
        self
    }
    fn right(mut self) -> Self {
        self.st
            .alignment_mut()
            .set_horizontal(HorizontalAlignmentValues::Right);
        self
    }
    fn center(mut self) -> Self {
        self.st
            .alignment_mut()
            .set_horizontal(HorizontalAlignmentValues::Center);
        self
    }
    fn fill(mut self, argb: &str) -> Self {
        self.st.set_background_color(argb);
        self
    }
    fn fmt(mut self, code: &str) -> Self {
        self.st.number_format_mut().set_format_code(code);
        self
    }
    fn done(self) -> Style {
        self.st
    }
}
