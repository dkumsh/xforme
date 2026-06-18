//! End-to-end test for the Excel-template engine: build a tiny template
//! workbook in memory, render a data sheet against it, and assert the
//! invariants — template tab removed, repeating rows expanded, placeholders
//! filled, and relative formulas row-shifted.

use umya_spreadsheet::Workbook;
use xforme::data;

/// Build a minimal 1-column-of-content template:
/// `A` = control tag, `B` = qty placeholder, `C` = `B*B`-ish line formula,
/// footer sums the detail band.
fn build_template(path: &std::path::Path) {
    let mut book = umya_spreadsheet::new_file();
    book.set_sheet_name(0, "Tmpl").unwrap();
    let ws = book.sheet_by_name_mut("Tmpl").unwrap();

    // header (fields: 1 = title text)
    ws.cell_mut("A1").set_value("header");
    ws.cell_mut("B1").set_value("${1}");

    // detail band: one repeating row, amount = qty * 10 via formula B{r}*10
    ws.cell_mut("A2").set_value("item");
    ws.cell_mut("B2").set_value("${1}"); // qty
    ws.cell_mut("C2").set_formula("B2*10");

    // footer: sum of the amount column across the detail band
    ws.cell_mut("A3").set_value("footer");
    ws.cell_mut("C3")
        .set_formula("SUM(C${firstrow}:C${lastrow})");

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

    // Footer landed on row 5 and its SUM spans the detail band rows 2..4.
    assert_eq!(ws.cell("C5").unwrap().formula(), "SUM(C2:C4)");

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
