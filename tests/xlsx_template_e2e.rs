//! End-to-end test for the Excel-template engine: build a tiny template
//! workbook in memory, render a data sheet against it, and assert the
//! invariants — template tab removed, repeating rows expanded, parameters
//! filled from comment markers, and relative formulas row-shifted.

use umya_spreadsheet::{Comment, Workbook, Worksheet};
use xforme::data;

/// Bind a parameter: real sample value in the cell, `#name` in its comment.
fn param(ws: &mut Worksheet, at: &str, sample: &str, name: &str) {
    ws.cell_mut(at).set_value(sample);
    let mut c = Comment::default();
    c.new_comment(at);
    c.set_text_string(format!("#{name}"));
    ws.add_comments(c);
}

/// Build a minimal template: column-A labels declare schemas, parameters are
/// comment-bound `#name` with real sample values, and the footer sums the
/// detail band via mixed anchoring.
fn build_template(path: &std::path::Path) {
    let mut book = umya_spreadsheet::new_file();
    book.set_sheet_name(0, "Tmpl").unwrap();
    let ws = book.sheet_by_name_mut("Tmpl").unwrap();

    // header band: schema declares one field, `title`.
    ws.cell_mut("A1").set_value("header(title)");
    param(ws, "B1", "Sample Title", "title");

    // detail band: schema `qty`; amount = qty * 10 via a real formula. Seed a
    // *stale* cached result (as Excel/LibreOffice would) to check it's cleared.
    ws.cell_mut("A2").set_value("item(qty)");
    param(ws, "B2", "1", "qty");
    ws.cell_mut("C2").set_formula("B2*10");
    ws.cell_mut("C2").set_formula_result_number(999.0);

    // footer: SUM over the detail band, mixed anchoring (start anchored).
    ws.cell_mut("A3").set_value("footer");
    ws.cell_mut("C3").set_formula("SUM(C$2:C2)");

    // A custom row height on the header row, to check it's carried to output.
    ws.row_dimension_mut(1).set_height(33.0);

    umya_spreadsheet::writer::xlsx::write(&book, path).unwrap();
}

fn read_back(path: &std::path::Path) -> Workbook {
    umya_spreadsheet::reader::xlsx::read(path).unwrap()
}

#[test]
fn renders_template_to_report() {
    let dir = std::env::temp_dir();
    let tpl = dir.join("xforme_test_tmpl.xlsx");
    let out = dir.join("xforme_test_report.xlsx");
    build_template(&tpl);

    // Three detail records -> the single detail template row repeats 3x.
    let raw = "#sheet\tTmpl\tReport\n\
               header\tMy Report\n\
               item\t2\n\
               item\t3\n\
               item\t4\n\
               footer\n\
               ##end\n";
    let sheets = data::parse(raw).unwrap();

    xforme::xlsx_template::render_to_file(tpl.as_path(), &sheets[0], &out).unwrap();

    let book = read_back(&out);

    // The template tab is gone; only the rendered "Report" sheet remains.
    assert!(
        book.sheet_by_name("Tmpl").is_err(),
        "template sheet should be removed"
    );
    let ws = book
        .sheet_by_name("Report")
        .expect("rendered sheet present");

    // Header placeholder filled.
    assert_eq!(ws.value("B1"), "My Report");

    // The custom header-row height was carried over.
    assert_eq!(ws.row_dimension(1).map(|r| r.height()), Some(33.0));

    // Detail band: rows 2,3,4 carry qty 2,3,4 with row-shifted formulas.
    assert_eq!(ws.value("B2"), "2");
    assert_eq!(ws.cell("C2").unwrap().formula(), "B2*10");
    assert_eq!(ws.cell("C3").unwrap().formula(), "B3*10");
    assert_eq!(ws.cell("C4").unwrap().formula(), "B4*10");

    // Footer landed on row 5; its SUM expanded over the rendered detail band:
    // the anchored start stays `C$2`, the relative end grew from C2 to C4.
    assert_eq!(ws.cell("C5").unwrap().formula(), "SUM(C$2:C4)");

    // The stale cached formula result was cleared (recomputes on open).
    assert_ne!(ws.cell("C2").unwrap().value(), "999");

    let _ = std::fs::remove_file(&tpl);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn warns_on_unmatched_data_label() {
    let dir = std::env::temp_dir();
    let tpl = dir.join("xforme_warn_tmpl.xlsx");
    build_template(&tpl);

    // `widget` matches no template label (the template has header/item/footer).
    let raw = "#sheet\tTmpl\tReport\n\
               header\tTitle\n\
               item\t1\n\
               widget\t9\n\
               footer\n\
               ##end\n";
    let sheet = &data::parse(raw).unwrap()[0];
    let bytes = std::fs::read(&tpl).unwrap();
    let (_book, warnings) =
        xforme::xlsx_template::render_with_warnings(bytes.as_slice(), sheet).unwrap();
    assert!(
        warnings.iter().any(|w| w.contains("widget")),
        "expected a warning about `widget`, got: {warnings:?}"
    );
    let _ = std::fs::remove_file(&tpl);
}

#[test]
fn renders_template_from_bytes() {
    // Build a template on disk, then feed its *bytes* to the engine — no file
    // path involved in rendering, and the output is collected as bytes too.
    let dir = std::env::temp_dir();
    let tpl = dir.join("xforme_bytes_tmpl.xlsx");
    build_template(&tpl);
    let template_bytes = std::fs::read(&tpl).unwrap();

    let raw = "#sheet\tTmpl\tReport\n\
               header\tIn-Memory\n\
               item\t5\n\
               footer\n\
               ##end\n";
    let sheets = data::parse(raw).unwrap();

    // Byte slice in, byte vec out.
    let out_bytes =
        xforme::xlsx_template::render_to_bytes(template_bytes.as_slice(), &sheets[0]).unwrap();
    assert!(!out_bytes.is_empty());

    // Round-trip the produced bytes back through the reader to verify content.
    let book =
        umya_spreadsheet::reader::xlsx::read_reader(std::io::Cursor::new(out_bytes), true).unwrap();
    assert!(
        book.sheet_by_name("Tmpl").is_err(),
        "template sheet removed"
    );
    let ws = book
        .sheet_by_name("Report")
        .expect("rendered sheet present");
    assert_eq!(ws.value("B1"), "In-Memory");
    assert_eq!(ws.value("B2"), "5");
    assert_eq!(ws.cell("C2").unwrap().formula(), "B2*10");

    let _ = std::fs::remove_file(&tpl);
}

#[test]
fn reads_rich_text_comment_markers() {
    // Excel/LibreOffice rewrite cell comments as rich-text runs on save. The
    // engine must still read the `#name` marker from that form (regression).
    use umya_spreadsheet::{Comment, CommentText, RichText, TextElement};

    let dir = std::env::temp_dir();
    let tpl = dir.join("xforme_richcomment_tmpl.xlsx");
    {
        let mut book = umya_spreadsheet::new_file();
        book.set_sheet_name(0, "Tmpl").unwrap();
        let ws = book.sheet_by_name_mut("Tmpl").unwrap();
        ws.cell_mut("A1").set_value("header(title)");
        ws.cell_mut("B1").set_value("PLACEHOLDER");

        // A comment whose text lives in rich-text runs (not a plain node).
        let mut element = TextElement::default();
        element.set_text("#title");
        let mut rich = RichText::default();
        rich.add_rich_text_elements(element);
        let mut text = CommentText::default();
        text.set_rich_text(rich);
        let mut comment = Comment::default();
        comment.new_comment("B1");
        comment.set_text(text);
        ws.add_comments(comment);

        umya_spreadsheet::writer::xlsx::write(&book, &tpl).unwrap();
    }

    let raw = "#sheet\tTmpl\tReport\nheader\tHello\n##end\n";
    let sheet = &data::parse(raw).unwrap()[0];
    let bytes = std::fs::read(&tpl).unwrap();
    let out = xforme::xlsx_template::render_to_bytes(bytes.as_slice(), sheet).unwrap();

    let book =
        umya_spreadsheet::reader::xlsx::read_reader(std::io::Cursor::new(out), true).unwrap();
    let ws = book.sheet_by_name("Report").unwrap();
    // The rich-text `#title` was resolved to the data, not left as the sample.
    assert_eq!(ws.value("B1"), "Hello");

    let _ = std::fs::remove_file(&tpl);
}

#[test]
fn preserves_and_grows_conditional_formatting() {
    // The whole point of clone-and-edit: features we don't touch (here a
    // conditional-formatting rule) survive, and umya's row insert grows the
    // rule's range with the expanding detail band.
    use umya_spreadsheet::{
        Comment, ConditionalFormatValues, ConditionalFormatting, ConditionalFormattingRule,
        SequenceOfReferences, Style,
    };

    let dir = std::env::temp_dir();
    let tpl = dir.join("xforme_cf_tmpl.xlsx");
    {
        let mut book = umya_spreadsheet::new_file();
        book.set_sheet_name(0, "T").unwrap();
        let ws = book.sheet_by_name_mut("T").unwrap();
        // 2-row detail band (row1/row2) with an amount in column B.
        for (cell, label) in [("A1", "row1(amount)"), ("A2", "row2(amount)")] {
            ws.cell_mut(cell).set_value(label);
        }
        for cell in ["B1", "B2"] {
            ws.cell_mut(cell).set_value("0");
            let mut c = Comment::default();
            c.new_comment(cell);
            c.set_text_string("#amount");
            ws.add_comments(c);
        }
        // A conditional-formatting rule over the band's amount column.
        let mut seq = SequenceOfReferences::default();
        seq.set_sqref("B1:B2");
        let mut style = Style::default();
        style.set_background_color("FFFFFF00");
        let mut rule = ConditionalFormattingRule::default();
        rule.set_type(ConditionalFormatValues::DuplicateValues);
        rule.set_priority(1);
        rule.set_style(style);
        let mut cf = ConditionalFormatting::default();
        cf.set_sequence_of_references(seq);
        cf.add_conditional_collection(rule);
        ws.add_conditional_formatting_collection(cf);

        umya_spreadsheet::writer::xlsx::write(&book, &tpl).unwrap();
    }

    // Four records -> band expands to 4 rows; the CF range should grow to B1:B4.
    let raw = "#sheet\tT\tOut\nrow1\t10\nrow2\t20\nrow1\t30\nrow2\t40\n##end\n";
    let sheet = &data::parse(raw).unwrap()[0];
    let bytes = std::fs::read(&tpl).unwrap();
    let out = xforme::xlsx_template::render_to_bytes(bytes.as_slice(), sheet).unwrap();

    let book =
        umya_spreadsheet::reader::xlsx::read_reader(std::io::Cursor::new(out), true).unwrap();
    let ws = book.sheet_by_name("Out").unwrap();
    let cf = ws.conditional_formatting_collection();
    assert_eq!(cf.len(), 1, "conditional formatting preserved");
    let sqref = cf[0].get_sequence_of_references().get_sqref();
    assert!(
        sqref.contains("B4"),
        "CF range grew with the band, got {sqref:?}"
    );

    let _ = std::fs::remove_file(&tpl);
}
