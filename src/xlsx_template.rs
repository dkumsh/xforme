//! Excel-template mode — the faithful xforme workflow.
//!
//! Unlike the declarative engine in [`crate::engine`], here the **template is a
//! real `.xlsx` workbook** designed in Excel/LibreOffice. The designer controls
//! every style, number format, merge and *formula*; this engine only injects
//! data and replicates the repeating rows.
//!
//! # Template convention
//!
//! * **Column A of each template row carries a visible control label** (the
//!   column is hidden in the output):
//!   * `header`  — emitted once, fields come from the `header` data record;
//!   * `footer`  — emitted once, fields come from the `footer` data record;
//!   * any other non-empty label (e.g. `row1`, `row2`) — a *detail* row. The
//!     contiguous run of detail template rows forms the **detail band**; the
//!     engine emits one output row per detail data record, choosing the
//!     template row whose label matches the record's label (so `row1`/`row2`
//!     alternating styles interleave in data order);
//!   * empty — a static row, emitted once verbatim.
//! * **Field-name schema** is declared once per label, in the column-A text:
//!   `header(date,receipt,customer,address)`. The first declaration of a label
//!   wins; bare `header` rows just attach to it. The schema maps `#name`
//!   parameters to positional data fields.
//! * **Parameters** are marked on a cell's **comment** — `#name` (resolved via
//!   the label's schema) or `#N` (1-based positional). The cell itself keeps a
//!   **real sample value**, so formulas compute and number formats preview while
//!   designing; at render the engine swaps in the data field. Excel's red
//!   comment markers make parameter cells visible at a glance.
//! * **Formulas** are authored natively in Excel and stay valid Excel. When a
//!   template row lands at a different output row, the engine shifts each
//!   *relative* cell reference's row by that row's delta, leaving `$`-anchored
//!   rows fixed — exactly like Excel's own copy/fill. So `=B7*D7` on template
//!   row 7 becomes `=B12*D12` on row 12.
//! * **Totals over the variable-length detail band** use ordinary Excel mixed
//!   anchoring: write `=SUM(E$7:E8)` in the footer, with the start row anchored
//!   to the first detail row and the end row relative to the last detail
//!   template row. As the band expands and the footer slides down, the relative
//!   end grows to cover every rendered detail row while the anchored start stays
//!   put. (This mirrors the original templateIt, which offset only the relative
//!   parts of references when replicating cells.)
//!
//! The output workbook gets a new sheet (named after the data stream's sheet
//! title) holding the rendered result, and the template sheet is removed.

use crate::data::Sheet;
use std::io::Cursor;
use std::path::Path;
use umya_spreadsheet::{Style, Workbook};

mod formula;
pub use formula::shift_rows;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Where a template `.xlsx` workbook comes from.
///
/// The engine can read a template either from a file on disk or straight from
/// an in-memory byte buffer (e.g. one served over the network, embedded with
/// [`include_bytes!`], or fetched from a database).
pub enum TemplateSource<'a> {
    /// Read the template from a file path.
    Path(&'a Path),
    /// Use an in-memory `.xlsx` byte buffer.
    Bytes(&'a [u8]),
}

impl<'a> From<&'a Path> for TemplateSource<'a> {
    fn from(p: &'a Path) -> Self {
        TemplateSource::Path(p)
    }
}

impl<'a> From<&'a [u8]> for TemplateSource<'a> {
    fn from(b: &'a [u8]) -> Self {
        TemplateSource::Bytes(b)
    }
}

impl<'a> From<&'a Vec<u8>> for TemplateSource<'a> {
    fn from(b: &'a Vec<u8>) -> Self {
        TemplateSource::Bytes(b.as_slice())
    }
}

/// Render a data sheet against a template, returning the populated workbook with
/// the template sheet removed.
///
/// `source` accepts anything convertible into a [`TemplateSource`], so both a
/// file path and a byte slice work:
///
/// ```no_run
/// use std::path::Path;
/// # use xforme::{data, xlsx_template::{self, TemplateSource}};
/// # fn demo(sheet: &data::Sheet, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
/// let from_file = xlsx_template::render(Path::new("template.xlsx"), sheet)?;
/// let from_mem  = xlsx_template::render(bytes, sheet)?;
/// # let _ = (from_file, from_mem);
/// # Ok(())
/// # }
/// ```
pub fn render<'a>(source: impl Into<TemplateSource<'a>>, sheet: &Sheet) -> Result<Workbook> {
    let mut book = read_template(source.into())?;

    let model = extract_template(&book, &sheet.template)?;
    let plan = plan_rows(&model, sheet);

    // The model is fully owned now, so we can mutate the workbook freely.
    book.remove_sheet_by_name(&sheet.template)
        .map_err(|e| format!("removing template sheet `{}`: {e:?}", sheet.template))?;

    // The template sheet is gone, so its title (or the data title) is free.
    let title = if sheet.title.trim().is_empty() {
        "Report"
    } else {
        sheet.title.trim()
    };
    write_output(&mut book, title, &model, &plan)?;
    Ok(book)
}

/// Render and save the report to `output_path`.
pub fn render_to_file<'a>(
    source: impl Into<TemplateSource<'a>>,
    sheet: &Sheet,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    let book = render(source, sheet)?;
    let path = output_path.as_ref();
    umya_spreadsheet::writer::xlsx::write(&book, path)
        .map_err(|e| format!("writing {}: {e:?}", path.display()))?;
    Ok(())
}

/// Render the report and return it as `.xlsx` bytes — the in-memory counterpart
/// of [`render_to_file`].
pub fn render_to_bytes<'a>(
    source: impl Into<TemplateSource<'a>>,
    sheet: &Sheet,
) -> Result<Vec<u8>> {
    let book = render(source, sheet)?;
    let mut buf = Vec::new();
    umya_spreadsheet::writer::xlsx::write_writer(&book, &mut buf)
        .map_err(|e| format!("serializing workbook: {e:?}"))?;
    Ok(buf)
}

/// Load a template workbook from either a path or a byte buffer.
fn read_template(source: TemplateSource) -> Result<Workbook> {
    match source {
        TemplateSource::Path(path) => umya_spreadsheet::reader::xlsx::read(path)
            .map_err(|e| format!("reading template {}: {e:?}", path.display()).into()),
        TemplateSource::Bytes(bytes) => {
            umya_spreadsheet::reader::xlsx::read_reader(Cursor::new(bytes), true)
                .map_err(|e| format!("reading template from {} bytes: {e:?}", bytes.len()).into())
        }
    }
}

// ---------------------------------------------------------------------------
// Template extraction
// ---------------------------------------------------------------------------

/// A parameter binding declared on a cell's *comment*. The cell keeps a real
/// sample value (so formulas compute and formatting previews); at render time
/// the engine replaces that value with the matching data field.
enum Param {
    /// `#name` — resolved against the row label's declared field-name schema.
    Named(String),
    /// `#N` — the N-th data field, 1-based.
    Indexed(usize),
}

/// A single template cell, with its style cloned out of the workbook.
struct TplCell {
    col: u32,
    /// The cell's literal value, or its formula text when `is_formula`.
    content: String,
    is_formula: bool,
    /// Set when the cell's comment marked it as a data parameter.
    param: Option<Param>,
    style: Style,
}

/// A template row: its control label plus content cells and any single-row merges.
struct TplRow {
    row: u32,
    /// The control label parsed from column A (`header`, `row1`, `footer`, …,
    /// or empty for a static row), with any `(field,names)` schema stripped off.
    label: String,
    cells: Vec<TplCell>,
    /// Horizontal merges on this row, as `(start_col, end_col)`.
    merges: Vec<(u32, u32)>,
}

/// The owned, workbook-independent template model.
struct TplModel {
    rows: Vec<TplRow>,
    /// `(column_number, width)` for every column that declares a width.
    col_widths: Vec<(u32, f64)>,
    /// Ordered field names declared per label, e.g.
    /// `header → [date, receipt, customer, address]`. Used to resolve `#name`
    /// parameters to positional data fields.
    schemas: std::collections::HashMap<String, Vec<String>>,
}

const TAG_COL: u32 = 1;

/// Parse a column-A control cell into `(label, optional field-name schema)`.
/// `"header(date,receipt,customer)"` → `("header", Some([date,receipt,customer]))`;
/// `"header"` → `("header", None)`.
fn parse_tag(raw: &str) -> (String, Option<Vec<String>>) {
    let raw = raw.trim();
    if let Some(open) = raw.find('(')
        && raw.ends_with(')')
    {
        let label = raw[..open].trim().to_string();
        let names = raw[open + 1..raw.len() - 1]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return (label, Some(names));
    }
    (raw.to_string(), None)
}

/// Parse a parameter marker out of a cell comment: `#name` → [`Param::Named`],
/// `#3` → [`Param::Indexed`]. Returns `None` if there is no `#token`.
fn parse_param(comment: &str) -> Option<Param> {
    let hash = comment.find('#')?;
    let token: String = comment[hash + 1..]
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if token.is_empty() {
        return None;
    }
    match token.parse::<usize>() {
        Ok(n) if n >= 1 => Some(Param::Indexed(n)),
        Ok(_) => None,
        Err(_) => Some(Param::Named(token)),
    }
}

fn extract_template(book: &Workbook, sheet_name: &str) -> Result<TplModel> {
    let ws = book
        .sheet_by_name(sheet_name)
        .map_err(|e| format!("template sheet `{sheet_name}` not found: {e:?}"))?;
    let (max_col, max_row) = ws.highest_column_and_row();

    // Index single-row merges by their row.
    let mut merges_by_row: std::collections::HashMap<u32, Vec<(u32, u32)>> = Default::default();
    for range in ws.merge_cells() {
        let (c1, r1, c2, r2) = parse_range(&range.range());
        if r1 == r2 {
            merges_by_row.entry(r1).or_default().push((c1, c2));
        }
    }

    // Index parameter markers by their (col, row), read from cell comments.
    let mut params_by_cell: std::collections::HashMap<(u32, u32), Param> = Default::default();
    for comment in ws.comments() {
        let text = comment
            .text()
            .text()
            .map(|t| t.value().to_string())
            .unwrap_or_default();
        if let Some(param) = parse_param(&text) {
            let co = comment.coordinate();
            params_by_cell.insert((co.col_num(), co.row_num()), param);
        }
    }

    let mut rows = Vec::new();
    let mut schemas: std::collections::HashMap<String, Vec<String>> = Default::default();
    for r in 1..=max_row {
        let (label, schema) = parse_tag(&ws.value(coord(TAG_COL, r)));
        // First declaration of a label's field schema wins.
        if let Some(names) = schema {
            schemas.entry(label.clone()).or_insert(names);
        }
        let mut cells = Vec::new();
        for c in (TAG_COL + 1)..=max_col {
            if let Some(cell) = ws.cell(coord(c, r)) {
                let is_formula = cell.is_formula();
                let content = if is_formula {
                    cell.formula().to_string()
                } else {
                    cell.value().to_string()
                };
                cells.push(TplCell {
                    col: c,
                    content,
                    is_formula,
                    param: params_by_cell.remove(&(c, r)),
                    style: cell.style().clone(),
                });
            }
        }
        rows.push(TplRow {
            row: r,
            label,
            cells,
            merges: merges_by_row.remove(&r).unwrap_or_default(),
        });
    }

    let mut col_widths = Vec::new();
    for c in 1..=max_col {
        if let Some(col) = ws.column_dimension_by_number(c) {
            col_widths.push((c, col.width()));
        }
    }

    Ok(TplModel {
        rows,
        col_widths,
        schemas,
    })
}

// ---------------------------------------------------------------------------
// Planning: map template rows + data records onto output rows
// ---------------------------------------------------------------------------

/// One planned output row: which template row produces it and which data fields
/// feed its placeholders.
struct Emit<'a> {
    tpl: &'a TplRow,
    fields: Vec<String>,
    out_row: u32,
}

struct Plan<'a> {
    emits: Vec<Emit<'a>>,
}

/// Labels that denote a once-only band rather than a repeating detail row.
fn is_singleton_label(label: &str) -> bool {
    matches!(label, "header" | "footer")
}

fn is_detail_label(label: &str) -> bool {
    !label.is_empty() && !is_singleton_label(label)
}

fn plan_rows<'a>(model: &'a TplModel, sheet: &Sheet) -> Plan<'a> {
    let record_fields = |label: &str| -> Vec<String> {
        sheet
            .records
            .iter()
            .find(|rec| rec.label == label)
            .map(|rec| rec.fields.clone())
            .unwrap_or_default()
    };
    let detail_records: Vec<&crate::data::Record> = sheet
        .records
        .iter()
        .filter(|rec| is_detail_label(&rec.label))
        .collect();

    let mut emits = Vec::new();
    let mut out_row: u32 = 1;

    let mut i = 0;
    while i < model.rows.len() {
        let row = &model.rows[i];
        if is_detail_label(&row.label) {
            // Gather the contiguous detail band.
            let band_start = i;
            while i < model.rows.len() && is_detail_label(&model.rows[i].label) {
                i += 1;
            }
            let band = &model.rows[band_start..i];
            for rec in &detail_records {
                let tpl = band
                    .iter()
                    .find(|tr| tr.label == rec.label)
                    .unwrap_or(&band[0]);
                emits.push(Emit {
                    tpl,
                    fields: rec.fields.clone(),
                    out_row,
                });
                out_row += 1;
            }
        } else {
            let fields = if is_singleton_label(&row.label) {
                record_fields(&row.label)
            } else {
                Vec::new()
            };
            emits.push(Emit {
                tpl: row,
                fields,
                out_row,
            });
            out_row += 1;
            i += 1;
        }
    }

    Plan { emits }
}

// ---------------------------------------------------------------------------
// Output writing
// ---------------------------------------------------------------------------

fn write_output(book: &mut Workbook, title: &str, model: &TplModel, plan: &Plan) -> Result<()> {
    let ws = book
        .new_sheet(title)
        .map_err(|e| format!("creating output sheet `{title}`: {e:?}"))?;

    for &(c, w) in &model.col_widths {
        ws.column_dimension_by_number_mut(c).set_width(w);
    }
    // Hide the control-tag column in the rendered report.
    ws.column_dimension_by_number_mut(TAG_COL).set_hidden(true);

    for emit in &plan.emits {
        let delta = emit.out_row as i64 - emit.tpl.row as i64;
        let schema = model.schemas.get(&emit.tpl.label);
        for tcell in &emit.tpl.cells {
            let at = coord(tcell.col, emit.out_row);
            let cell = ws.cell_mut(at.as_str());
            cell.set_style(tcell.style.clone());

            if tcell.is_formula {
                // Formulas are valid Excel; just shift relative references.
                cell.set_formula(formula::shift_rows(&tcell.content, delta));
            } else if let Some(param) = &tcell.param {
                // Replace the sample value with the resolved data field.
                cell.set_value(resolve_param(param, schema, &emit.fields));
            } else {
                // Static cell — keep the designer's literal value.
                cell.set_value(tcell.content.clone());
            }
        }
        for &(c1, c2) in &emit.tpl.merges {
            let range = format!("{}:{}", coord(c1, emit.out_row), coord(c2, emit.out_row));
            ws.add_merge_cells(range);
        }
    }

    Ok(())
}

/// Resolve a parameter to its data value: `#name` via the label's schema,
/// `#N` positionally. Missing bindings resolve to the empty string.
fn resolve_param(param: &Param, schema: Option<&Vec<String>>, fields: &[String]) -> String {
    let index = match param {
        Param::Indexed(n) => Some(n - 1),
        Param::Named(name) => schema.and_then(|names| names.iter().position(|x| x == name)),
    };
    index
        .and_then(|i| fields.get(i))
        .cloned()
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Coordinate helpers
// ---------------------------------------------------------------------------

/// Convert a 1-based `(col, row)` to an A1-style coordinate string.
fn coord(col: u32, row: u32) -> String {
    format!("{}{}", column_letter(col), row)
}

fn column_letter(mut col: u32) -> String {
    let mut s = Vec::new();
    while col > 0 {
        let rem = ((col - 1) % 26) as u8;
        s.push(b'A' + rem);
        col = (col - 1) / 26;
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}

fn column_index(letters: &str) -> u32 {
    let mut idx = 0u32;
    for ch in letters.bytes() {
        idx = idx * 26 + (ch.to_ascii_uppercase() - b'A' + 1) as u32;
    }
    idx
}

/// Parse `"B1:E1"` (or `"B1"`) into `(start_col, start_row, end_col, end_row)`.
fn parse_range(range: &str) -> (u32, u32, u32, u32) {
    let (a, b) = match range.split_once(':') {
        Some((a, b)) => (a, b),
        None => (range, range),
    };
    let (c1, r1) = parse_cell_ref(a);
    let (c2, r2) = parse_cell_ref(b);
    (c1, r1, c2, r2)
}

fn parse_cell_ref(s: &str) -> (u32, u32) {
    let s = s.replace('$', "");
    let split = s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len());
    let (letters, digits) = s.split_at(split);
    (column_index(letters), digits.parse().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_letters_round_trip() {
        for (n, s) in [
            (1, "A"),
            (2, "B"),
            (26, "Z"),
            (27, "AA"),
            (28, "AB"),
            (53, "BA"),
        ] {
            assert_eq!(column_letter(n), s);
            assert_eq!(column_index(s), n);
        }
    }

    #[test]
    fn parses_ranges() {
        assert_eq!(parse_range("B1:E1"), (2, 1, 5, 1));
        assert_eq!(parse_range("E10"), (5, 10, 5, 10));
        assert_eq!(parse_range("$A$3:$C$3"), (1, 3, 3, 3));
    }

    #[test]
    fn parses_column_a_tag_and_schema() {
        let (label, schema) = parse_tag("header(date, receipt, customer)");
        assert_eq!(label, "header");
        assert_eq!(
            schema,
            Some(vec!["date".into(), "receipt".into(), "customer".into()])
        );

        assert_eq!(parse_tag("row1"), ("row1".to_string(), None));
        assert_eq!(parse_tag("  footer  "), ("footer".to_string(), None));
    }

    #[test]
    fn parses_param_markers() {
        assert!(matches!(parse_param("#price"), Some(Param::Named(n)) if n == "price"));
        assert!(matches!(
            parse_param("see #2 here"),
            Some(Param::Indexed(2))
        ));
        assert!(parse_param("no marker").is_none());
        assert!(parse_param("#").is_none());
    }

    #[test]
    fn resolves_named_and_indexed_params() {
        let schema = vec!["date".to_string(), "receipt".to_string()];
        let fields = vec!["1/5/2009".to_string(), "22215".to_string()];
        assert_eq!(
            resolve_param(&Param::Named("receipt".into()), Some(&schema), &fields),
            "22215"
        );
        assert_eq!(
            resolve_param(&Param::Indexed(1), Some(&schema), &fields),
            "1/5/2009"
        );
        // Unknown name or out-of-range index → empty.
        assert_eq!(
            resolve_param(&Param::Named("nope".into()), Some(&schema), &fields),
            ""
        );
        assert_eq!(resolve_param(&Param::Indexed(9), None, &fields), "");
    }
}
