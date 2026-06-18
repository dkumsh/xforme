// Builds the bundled sample Sales Receipt template workbook.
//
// This stands in for the workbook a *designer* would normally craft by hand in
// Excel or LibreOffice. It exercises everything the Excel-template engine
// preserves: column-A control tags, fonts/bold, alignment, fills, number
// formats, merged cells, and native Excel formulas (line totals, a SUM over the
// variable-length detail band, tax and grand total).
//
// It is exposed as a library helper and used by the `make_sample_template`
// binary to (re)generate the committed `templates/sales_receipt_template.xlsx`
// asset, which the demo binary then embeds with `include_bytes!`.

use umya_spreadsheet::{HorizontalAlignmentValues, Style};

const SHADE: &str = "FFF2F2F2"; // banded row fill (ARGB)
const HEAD_FILL: &str = "FFD9E1F2"; // column-header fill
const CURRENCY: &str = "$#,##0.00";

/// Build the sample Sales Receipt template and return it as `.xlsx` bytes.
pub fn sample_sales_receipt_template_bytes() -> Vec<u8> {
    let mut book = umya_spreadsheet::new_file();
    book.set_sheet_name(0, "SalesReceipt")
        .expect("rename sheet");
    let ws = book.sheet_by_name_mut("SalesReceipt").expect("sheet");

    // Column widths: A is the (hidden-on-output) control-tag column.
    ws.column_dimension_mut("A").set_width(4.0);
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

    // Rows 2-4 — header band (fields: 1=date, 2=receipt, 3=customer, 4=address).
    set(ws, "A2", "header", plain());
    set(ws, "B2", "Receipt #:", sb().bold().done());
    set(ws, "C2", "${2}", plain());
    set(ws, "D2", "Date:", sb().bold().right().done());
    set(ws, "E2", "${1}", sb().right().done());

    set(ws, "A3", "header", plain());
    set(ws, "B3", "Sold To:", sb().bold().done());
    set(ws, "C3", "${3}", plain());
    ws.add_merge_cells("C3:E3");

    set(ws, "A4", "header", plain());
    set(ws, "C4", "${4}", plain());
    ws.add_merge_cells("C4:E4");

    // Row 5 — spacer.

    // Row 6 — column headers.
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

    // Rows 7-8 — the detail band: row1 (plain) and row2 (shaded). Fields:
    // 1=seq, 2=qty, 3=desc, 4=price. Amount is a native Excel formula.
    set(ws, "A7", "row1", plain());
    set(ws, "B7", "${2}", sb().right().done());
    set(ws, "C7", "${3}", plain());
    set(ws, "D7", "${4}", sb().right().fmt(CURRENCY).done());
    setf(ws, "E7", "B7*D7", sb().right().fmt(CURRENCY).done());

    set(ws, "A8", "row2", plain());
    set(ws, "B8", "${2}", sb().right().fill(SHADE).done());
    set(ws, "C8", "${3}", sb().fill(SHADE).done());
    set(
        ws,
        "D8",
        "${4}",
        sb().right().fill(SHADE).fmt(CURRENCY).done(),
    );
    setf(
        ws,
        "E8",
        "B8*D8",
        sb().right().fill(SHADE).fmt(CURRENCY).done(),
    );

    // Row 9 — spacer.

    // Rows 10-12 — footer band. The subtotal sums the detail band via the
    // ${firstrow}/${lastrow} markers; tax and total reference the cells above.
    set(ws, "A10", "footer", plain());
    set(ws, "D10", "Subtotal:", sb().bold().right().done());
    setf(
        ws,
        "E10",
        "SUM(E${firstrow}:E${lastrow})",
        sb().right().fmt(CURRENCY).done(),
    );

    set(ws, "A11", "footer", plain());
    set(ws, "C11", "Sales Tax", sb().right().done());
    set(ws, "D11", "${1}", sb().right().fmt("0.00%").done()); // tax rate
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

/// Set a cell's literal/placeholder value and style.
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
