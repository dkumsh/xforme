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

    // detail band: schema `qty`; amount = qty * 10 via a real formula.
    ws.cell_mut("A2").set_value("item(qty)");
    param(ws, "B2", "1", "qty");
    ws.cell_mut("C2").set_formula("B2*10");

    // footer: SUM over the detail band, mixed anchoring (start anchored).
    ws.cell_mut("A3").set_value("footer");
    ws.cell_mut("C3").set_formula("SUM(C$2:C2)");

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

    // Detail band: rows 2,3,4 carry qty 2,3,4 with row-shifted formulas.
    assert_eq!(ws.value("B2"), "2");
    assert_eq!(ws.cell("C2").unwrap().formula(), "B2*10");
    assert_eq!(ws.cell("C3").unwrap().formula(), "B3*10");
    assert_eq!(ws.cell("C4").unwrap().formula(), "B4*10");

    // Footer landed on row 5; its SUM expanded over the rendered detail band:
    // the anchored start stays `C$2`, the relative end grew from C2 to C4.
    assert_eq!(ws.cell("C5").unwrap().formula(), "SUM(C$2:C4)");

    let _ = std::fs::remove_file(&tpl);
    let _ = std::fs::remove_file(&out);
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
