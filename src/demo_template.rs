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

use umya_spreadsheet::structs::drawing::charts::{CategoryAxisData, StringReference};
use umya_spreadsheet::structs::drawing::spreadsheet::MarkerType;
use umya_spreadsheet::{
    BorderStyleValues, Chart, ChartType, Color, Comment, ConditionalFormatValueObject,
    ConditionalFormatValueObjectValues, ConditionalFormatValues, ConditionalFormatting,
    ConditionalFormattingOperatorValues, ConditionalFormattingRule, DataBar,
    HorizontalAlignmentValues, SequenceOfReferences, Style, Worksheet,
};

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

// ---- Portfolio Statement: the showcase template -------------------------------

const LOGO_PNG: &[u8] = include_bytes!("../templates/northwind_logo.png");

const NAVY: &str = "FF1F3A5F"; // brand navy — header fill & title
const TEAL: &str = "FF26A69A"; // brand accent
const GREY: &str = "FF5A6473"; // muted labels
const BAND: &str = "FFF4F7FB"; // zebra-striping for alternate rows
const WHITE: &str = "FFFFFFFF";
const GAIN: &str = "FF1E8449"; // positive figures (green)
const GAIN_FILL: &str = "FFE9F7EF";
const LOSS: &str = "FFC0392B"; // negative figures (red)
const LOSS_FILL: &str = "FFFCECEA";
const PCT: &str = "0.0%";
const INT: &str = "#,##0";

/// Build the showcase **Portfolio Statement** template and return it as `.xlsx`
/// bytes. Beyond what the Sales Receipt covers (bands, comment-bound `#name`
/// parameters, row-shifted formulas, a growing mixed-anchor `SUM`), this one
/// exercises the features the clone-and-edit engine preserves *and grows* with
/// the data: an embedded **logo image**, **conditional formatting** (red/green
/// on the gain column), **in-cell data bars** (a histogram of market value),
/// two **column charts** whose series ranges expand with the holdings, plus a
/// landscape **print setup**. The detail band is `row1`/`row2` so holdings
/// zebra-stripe in data order.
pub fn sample_portfolio_statement_template_bytes() -> Vec<u8> {
    let mut book = umya_spreadsheet::new_file();
    book.set_sheet_name(0, "Portfolio").expect("rename sheet");
    let ws = book.sheet_by_name_mut("Portfolio").expect("sheet");

    // Column A carries the control labels (hidden in the output). B..G are the
    // printed columns of the holdings table.
    ws.column_dimension_mut("A").set_width(26.0);
    ws.column_dimension_mut("B").set_width(20.0);
    ws.column_dimension_mut("C").set_width(8.0);
    ws.column_dimension_mut("D").set_width(15.0);
    ws.column_dimension_mut("E").set_width(15.0);
    ws.column_dimension_mut("F").set_width(15.0);
    ws.column_dimension_mut("G").set_width(11.0);

    // Rows 1-2 — masthead. The logo floats over the left columns; the title and
    // subtitle sit on the right. (Static rows: blank column A, emitted verbatim.)
    ws.row_dimension_mut(1).set_height(30.0);
    ws.row_dimension_mut(2).set_height(18.0);
    set(
        ws,
        "E1",
        "PORTFOLIO STATEMENT",
        sb().bold().size(18.0).right().color(NAVY).done(),
    );
    ws.add_merge_cells("E1:G1");
    set(
        ws,
        "E2",
        "Monthly Holdings Summary",
        sb().size(10.0).right().color(GREY).done(),
    );
    ws.add_merge_cells("E2:G2");
    add_logo(ws, "B1");

    // Row 3 — spacer.

    // Rows 4-5 — header band. The schema names the four header fields once; the
    // cells hold obvious placeholder samples bound by `#name` comments.
    set(ws, "A4", "header(account,holder,period,asof)", plain());
    set(ws, "B4", "Account No.", sb().bold().color(GREY).done());
    param(ws, "C4", "0000-000000", "account", plain());
    set(
        ws,
        "E4",
        "Statement Period",
        sb().bold().right().color(GREY).done(),
    );
    param(ws, "F4", "Month YYYY", "period", sb().right().done());
    ws.add_merge_cells("F4:G4");

    set(ws, "A5", "header", plain());
    set(ws, "B5", "Account Holder", sb().bold().color(GREY).done());
    param(ws, "C5", "Holder Name", "holder", plain());
    ws.add_merge_cells("C5:D5");
    set(ws, "E5", "Prepared", sb().bold().right().color(GREY).done());
    param(ws, "F5", "MM/DD/YYYY", "asof", sb().right().done());
    ws.add_merge_cells("F5:G5");

    // Row 6 — spacer.

    // Row 7 — column headers (static), navy fill with white bold text.
    let head = || sb().bold().color(WHITE).fill(NAVY);
    set(ws, "B7", "Security", head().done());
    set(ws, "C7", "Qty", head().right().done());
    set(ws, "D7", "Cost Basis", head().right().done());
    set(ws, "E7", "Market Value", head().right().done());
    set(ws, "F7", "Gain / Loss", head().right().done());
    set(ws, "G7", "Return %", head().right().done());

    // Rows 8-9 — detail band: row1 (plain) and row2 (striped). Gain and Return
    // are native Excel formulas over the data cells; they row-shift as the band
    // is replicated. Column A declares the field schema for each.
    detail_row(ws, 8, None);
    detail_row(ws, 9, Some(BAND));

    // Row 10 — spacer.

    // Row 11 — footer totals. Cost/Market/Gain sum the detail band with mixed
    // anchoring (`D$8:D9`): the start row is anchored to the first holding, the
    // end row is relative to the last detail template row, so the sum grows to
    // cover every rendered holding. Total return is gain / cost on this row.
    set(ws, "A11", "footer", plain());
    set(
        ws,
        "B11",
        "TOTAL",
        sb().bold().color(NAVY).top_border(NAVY).done(),
    );
    for col in ["C", "D", "E", "F", "G"] {
        // carry the top border across the whole totals row
        set(ws, &format!("{col}11"), "", sb().top_border(NAVY).done());
    }
    let total_money = || {
        sb().bold()
            .right()
            .color(NAVY)
            .fmt(CURRENCY)
            .top_border(NAVY)
    };
    setf(ws, "D11", "SUM(D$8:D9)", total_money().done());
    setf(ws, "E11", "SUM(E$8:E9)", total_money().done());
    setf(ws, "F11", "SUM(F$8:F9)", total_money().done());
    setf(
        ws,
        "G11",
        "F11/D11",
        sb().bold()
            .right()
            .color(NAVY)
            .fmt(PCT)
            .top_border(NAVY)
            .done(),
    );

    // Conditional formatting over the detail band (grows with it):
    //  * Gain/Loss column — green when positive, red when negative.
    //  * Market Value column — a teal in-cell data bar (a histogram per row).
    add_gain_loss_rules(ws, "F8:F9");
    add_market_data_bar(ws, "E8:E9");

    // Two column charts below the table. Their series reference the detail band,
    // so the engine grows the ranges as holdings are added.
    add_column_chart(
        ws,
        "Market Value by Holding",
        "B13",
        "G29",
        "Portfolio!$E$8:$E$9",
        "Portfolio!$B$8:$B$9",
    );
    add_column_chart(
        ws,
        "Gain / Loss by Holding",
        "B31",
        "G47",
        "Portfolio!$F$8:$F$9",
        "Portfolio!$B$8:$B$9",
    );

    // NOTE: we deliberately do *not* set a page setup here. umya 3.0 has a
    // relationship-id bug — `write_print_settings` reserves a rel id whenever
    // page setup has any parameter, but only emits the printer-settings
    // relationship when a binary blob is present (which it never is here). That
    // off-by-one shifts the `<drawing>` r:id past its relationship, and Excel/
    // LibreOffice then silently drop the *entire* drawing layer (logo + charts).
    // Leaving page setup unset keeps the drawings intact; the statement still
    // prints fine in portrait.

    let mut buf = Vec::new();
    umya_spreadsheet::writer::xlsx::write_writer(&book, &mut buf).expect("serialize template");
    buf
}

/// One holding row of the detail band at `row`, optionally striped with `fill`.
fn detail_row(ws: &mut Worksheet, row: u32, fill: Option<&str>) {
    let tint = |s: StyleBuilder| match fill {
        Some(argb) => s.fill(argb),
        None => s,
    };
    let label = if fill.is_some() { "row2" } else { "row1" };
    set(
        ws,
        &format!("A{row}"),
        &format!("{label}(symbol,qty,cost,market)"),
        plain(),
    );
    param(
        ws,
        &format!("B{row}"),
        "TICKER",
        "symbol",
        tint(sb()).done(),
    );
    param(
        ws,
        &format!("C{row}"),
        "0",
        "qty",
        tint(sb().right().fmt(INT)).done(),
    );
    param(
        ws,
        &format!("D{row}"),
        "0",
        "cost",
        tint(sb().right().fmt(CURRENCY)).done(),
    );
    param(
        ws,
        &format!("E{row}"),
        "0",
        "market",
        tint(sb().right().fmt(CURRENCY)).done(),
    );
    setf(
        ws,
        &format!("F{row}"),
        &format!("E{row}-D{row}"),
        tint(sb().right().fmt(CURRENCY)).done(),
    );
    setf(
        ws,
        &format!("G{row}"),
        &format!("F{row}/D{row}"),
        tint(sb().right().fmt(PCT)).done(),
    );
}

/// Embed the brand logo, anchored at `coord`. Dimensions are passed explicitly
/// (in px) so the on-sheet size is independent of the source resolution.
fn add_logo(ws: &mut Worksheet, coord: &str) {
    let mut marker = MarkerType::default();
    marker.set_coordinate(coord);
    let mut image = umya_spreadsheet::Image::default();
    image.new_image_with_dimensions(34, 200, "northwind_logo.png", LOGO_PNG.to_vec(), marker);
    ws.add_image(image);
}

/// Green-on-positive / red-on-negative cell rules over `range` (the gain column).
fn add_gain_loss_rules(ws: &mut Worksheet, range: &str) {
    let rule = |op: ConditionalFormattingOperatorValues, font: &str, fill: &str, priority: i32| {
        let mut style = Style::default();
        style.font_mut().set_bold(true);
        style.font_mut().color_mut().set_argb_str(font);
        style.set_background_color(fill);
        // The comparison value `0` goes in the rule's <formula> child.
        let mut formula = umya_spreadsheet::Formula::default();
        formula.set_string_value("0");
        let mut r = ConditionalFormattingRule::default();
        r.set_type(ConditionalFormatValues::CellIs);
        r.set_operator(op);
        r.set_formula(formula);
        r.set_priority(priority);
        r.set_style(style);
        r
    };
    let mut seq = SequenceOfReferences::default();
    seq.set_sqref(range);
    let mut cf = ConditionalFormatting::default();
    cf.set_sequence_of_references(seq);
    cf.add_conditional_collection(rule(
        ConditionalFormattingOperatorValues::LessThan,
        LOSS,
        LOSS_FILL,
        1,
    ));
    cf.add_conditional_collection(rule(
        ConditionalFormattingOperatorValues::GreaterThan,
        GAIN,
        GAIN_FILL,
        2,
    ));
    ws.add_conditional_formatting_collection(cf);
}

/// A teal in-cell data bar over `range` (the market-value column) — a per-row
/// histogram that scales from the smallest to the largest holding.
fn add_market_data_bar(ws: &mut Worksheet, range: &str) {
    let cfvo = |kind: ConditionalFormatValueObjectValues| {
        let mut o = ConditionalFormatValueObject::default();
        o.set_type(kind);
        o
    };
    let mut bar = DataBar::default();
    bar.add_cfvo_collection(cfvo(ConditionalFormatValueObjectValues::Min));
    bar.add_cfvo_collection(cfvo(ConditionalFormatValueObjectValues::Max));
    let mut color = Color::default();
    color.set_argb_str(TEAL);
    bar.add_color_collection(color);

    let mut rule = ConditionalFormattingRule::default();
    rule.set_type(ConditionalFormatValues::DataBar);
    rule.set_priority(3);
    rule.set_data_bar(bar);

    let mut seq = SequenceOfReferences::default();
    seq.set_sqref(range);
    let mut cf = ConditionalFormatting::default();
    cf.set_sequence_of_references(seq);
    cf.add_conditional_collection(rule);
    ws.add_conditional_formatting_collection(cf);
}

/// Add a column chart titled `title`, anchored between `from`/`to` cells, with
/// the value series `values` and category labels `categories` (cell-range
/// strings like `Portfolio!$E$8:$E$9`).
fn add_column_chart(
    ws: &mut Worksheet,
    title: &str,
    from: &str,
    to: &str,
    values: &str,
    categories: &str,
) {
    let mut from_marker = MarkerType::default();
    from_marker.set_coordinate(from);
    let mut to_marker = MarkerType::default();
    to_marker.set_coordinate(to);

    let mut chart = Chart::default();
    chart.new_chart(&ChartType::BarChart, from_marker, to_marker, vec![values]);
    chart.set_series_title(vec![title]);

    // Attach category labels (the security column) to the value series.
    for series in chart.area_chart_series_list_mut().area_chart_series_mut() {
        let mut formula = umya_spreadsheet::structs::drawing::charts::Formula::default();
        formula.set_address_str(categories);
        let mut sref = StringReference::default();
        sref.set_formula(formula);
        let mut cad = CategoryAxisData::default();
        cad.set_string_reference(sref);
        series.set_category_axis_data(cad);
    }
    ws.add_chart(chart);
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
    fn color(mut self, argb: &str) -> Self {
        self.st.font_mut().color_mut().set_argb_str(argb);
        self
    }
    /// A thin top border in `argb` — used to underline the totals row.
    fn top_border(mut self, argb: &str) -> Self {
        let mut color = Color::default();
        color.set_argb_str(argb);
        let top = self.st.borders_mut().top_mut();
        top.set_style(BorderStyleValues::Thin);
        top.set_color(color);
        self
    }
    fn done(self) -> Style {
        self.st
    }
}
