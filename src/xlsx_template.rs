//! Excel-template mode — the faithful xforme workflow.
//!
//! The **template is a real `.xlsx` workbook** designed in Excel/LibreOffice.
//! The designer controls every style, number format, merge and *formula*; this
//! engine only injects data and replicates the repeating rows.
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
//! The engine **edits the template sheet in place** (rather than building a new
//! one): it resizes the detail band with row inserts/removes, fills parameters,
//! hides the control column, strips the directive comments, and renames the
//! sheet to the output title. Keeping the sheet means everything the designer
//! put in the workbook that we don't touch — conditional formatting, images,
//! charts, data validations, print setup, frozen panes — is preserved, and
//! `umya`'s insert/remove adjusts the spanning ranges (a footer `SUM`, a
//! conditional-format range) along with the band. Cached formula results are
//! cleared so Excel/LibreOffice recompute on open.

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

/// Render a data sheet against a template, returning the populated workbook
/// (the template sheet edited in place and renamed to the output title).
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
    Ok(render_with_warnings(source, sheet)?.0)
}

/// Like [`render`], but also returns non-fatal **warnings** (e.g. a data record
/// whose label matches no template row, so its data is silently unused).
pub fn render_with_warnings<'a>(
    source: impl Into<TemplateSource<'a>>,
    sheet: &Sheet,
) -> Result<(Workbook, Vec<String>)> {
    let mut book = read_template(source.into())?;

    // Snapshot the template structure, then *edit the template sheet in place*.
    // Keeping the sheet (rather than building a new one) preserves everything
    // umya models that we don't touch — conditional formatting, images, charts,
    // data validations, print setup, frozen panes, column widths, styles.
    let model = extract_template(&book, &sheet.template)?;
    let warnings = collect_warnings(&model, sheet);
    fill_in_place(&mut book, sheet, &model)?;
    Ok((book, warnings))
}

/// Render and save the report to `output_path`, returning any [warnings].
///
/// [warnings]: render_with_warnings
pub fn render_to_file<'a>(
    source: impl Into<TemplateSource<'a>>,
    sheet: &Sheet,
    output_path: impl AsRef<Path>,
) -> Result<Vec<String>> {
    let (book, warnings) = render_with_warnings(source, sheet)?;
    let path = output_path.as_ref();
    umya_spreadsheet::writer::xlsx::write(&book, path)
        .map_err(|e| format!("writing {}: {e:?}", path.display()))?;
    Ok(warnings)
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
    /// Explicit row height, if the designer set one (copied to the output row).
    height: Option<f64>,
}

/// The owned, workbook-independent template model.
struct TplModel {
    rows: Vec<TplRow>,
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

/// Read a comment's text, handling both **plain** comments (as xforme writes
/// them) and **rich-text** comments (as Excel/LibreOffice rewrite them on save —
/// the text then lives in formatted runs, not a plain node).
fn comment_text(comment: &umya_spreadsheet::Comment) -> String {
    let ct = comment.text();
    if let Some(t) = ct.text() {
        let s = t.value();
        if !s.is_empty() {
            return s.to_string();
        }
    }
    if let Some(rt) = ct.rich_text() {
        return rt.text().into_owned();
    }
    String::new()
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
        if let Some(param) = parse_param(&comment_text(comment)) {
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
            height: ws.row_dimension(r).map(|rd| rd.height()),
        });
    }

    Ok(TplModel { rows, schemas })
}

// ---------------------------------------------------------------------------
// Filling: edit the (kept) template sheet in place
// ---------------------------------------------------------------------------

/// Labels that denote a once-only band rather than a repeating detail row.
fn is_singleton_label(label: &str) -> bool {
    matches!(label, "header" | "footer")
}

fn is_detail_label(label: &str) -> bool {
    !label.is_empty() && !is_singleton_label(label)
}

/// Non-fatal warnings: data records whose label matches no template row (their
/// data is silently unused — usually a typo or a template/data mismatch).
fn collect_warnings(model: &TplModel, sheet: &Sheet) -> Vec<String> {
    use std::collections::HashSet;
    let template_labels: HashSet<&str> = model
        .rows
        .iter()
        .map(|r| r.label.as_str())
        .filter(|l| !l.is_empty())
        .collect();
    let mut warnings = Vec::new();
    let mut reported: HashSet<&str> = HashSet::new();
    for rec in &sheet.records {
        if !template_labels.contains(rec.label.as_str()) && reported.insert(rec.label.as_str()) {
            warnings.push(format!(
                "data label `{}` has no matching template row — those records are ignored",
                rec.label
            ));
        }
    }
    warnings
}

/// The data fields (positional + named) of the record with `label`, if any.
fn record_data(
    sheet: &Sheet,
    label: &str,
) -> (Vec<String>, std::collections::BTreeMap<String, String>) {
    sheet
        .records
        .iter()
        .find(|rec| rec.label == label)
        .map(|rec| (rec.fields.clone(), rec.named.clone()))
        .unwrap_or_default()
}

/// The contiguous run of detail template rows as inclusive sheet rows
/// `(start, end, height)`, or `None` if the template has no detail rows.
fn detail_band(model: &TplModel) -> Option<(u32, u32, usize)> {
    let rows: Vec<&TplRow> = model
        .rows
        .iter()
        .filter(|r| is_detail_label(&r.label))
        .collect();
    rows.first()
        .map(|f| (f.row, rows[rows.len() - 1].row, rows.len()))
}

/// Render by editing the kept template sheet in place: resize the detail band to
/// the data, fill parameters, drop the control column and directive comments,
/// and rename the sheet to the output title. Everything else the designer put in
/// the workbook (conditional formatting, images, charts, print setup, …) is left
/// untouched, and `umya`'s row insert/remove adjusts the spanning ranges.
fn fill_in_place(book: &mut Workbook, sheet: &Sheet, model: &TplModel) -> Result<()> {
    let band = detail_band(model);
    let detail_records: Vec<&crate::data::Record> = sheet
        .records
        .iter()
        .filter(|r| is_detail_label(&r.label))
        .collect();
    let m = detail_records.len();
    // How far rows below the band move after the resize.
    let shift = band.map_or(0, |(_, _, h)| m as i64 - h as i64);
    let band_end = band.map(|(_, e, _)| e);
    let detail_rows: Vec<&TplRow> = model
        .rows
        .iter()
        .filter(|r| is_detail_label(&r.label))
        .collect();

    let ws = book
        .sheet_by_name_mut(&sheet.template)
        .map_err(|e| format!("template sheet `{}` not found: {e:?}", sheet.template))?;

    // 1. Resize the detail band to the record count. Insert *inside* the band so
    //    spanning ranges (a footer SUM, conditional formatting) grow with it.
    if let Some((start, end, h)) = band {
        let k = m as i64 - h as i64;
        if k > 0 {
            ws.insert_new_row(end, k as u32);
        } else if k < 0 {
            ws.remove_row(start, (-k) as u32);
        }
    }

    // 2. The control column and the parameter comments don't belong in output.
    ws.column_dimension_by_number_mut(TAG_COL).set_hidden(true);
    ws.comments_mut().clear();

    // 3. Fill parameters on the once-only header/footer rows, at their post-resize
    //    positions (rows below the band moved by `shift`).
    for trow in &model.rows {
        if !is_singleton_label(&trow.label) {
            continue;
        }
        let cur = match band_end {
            Some(end) if trow.row > end => (trow.row as i64 + shift) as u32,
            _ => trow.row,
        };
        let (fields, named) = record_data(sheet, &trow.label);
        let schema = model.schemas.get(&trow.label);
        for cell in &trow.cells {
            if let Some(param) = &cell.param {
                ws.cell_mut(coord(cell.col, cur).as_str())
                    .set_value(resolve_param(param, schema, &fields, &named));
            }
        }
    }

    // 4. Rewrite the detail band: one row per record, banding by label, formulas
    //    row-shifted, params resolved, merges and heights re-applied.
    if let Some((start, _, h)) = band
        && m > 0
    {
        // Drop any merges inside the resized band; we re-add them per row.
        let (lo, hi) = (start, start + m as u32 - 1);
        ws.merge_cells_mut().retain(|rng| {
            let (_, r1, _, r2) = parse_range(&rng.range());
            !(r1 >= lo && r2 <= hi)
        });
        for (i, rec) in detail_records.iter().enumerate() {
            let target = start + i as u32;
            let src = detail_rows
                .iter()
                .find(|r| r.label == rec.label)
                .copied()
                .unwrap_or(detail_rows[i % h]);
            let delta = target as i64 - src.row as i64;
            let schema = model.schemas.get(&src.label);
            if let Some(height) = src.height
                && height > 0.0
            {
                ws.row_dimension_mut(target).set_height(height);
            }
            for cell in &src.cells {
                let c = ws.cell_mut(coord(cell.col, target).as_str());
                c.set_style(cell.style.clone());
                if cell.is_formula {
                    c.set_formula(formula::shift_rows(&cell.content, delta));
                } else if let Some(param) = &cell.param {
                    c.set_value(resolve_param(param, schema, &rec.fields, &rec.named));
                } else {
                    c.set_value(cell.content.clone());
                }
            }
            for &(c1, c2) in &src.merges {
                ws.add_merge_cells(format!("{}:{}", coord(c1, target), coord(c2, target)));
            }
        }
    }

    // 5. Drop cached formula results. The cloned template may carry stale cached
    //    values (e.g. computed from the sample data, or by a prior Excel/
    //    LibreOffice save); clearing them forces a recompute on open.
    for cell in ws.cells_mut() {
        if cell.is_formula() {
            cell.set_formula_result_default("");
        }
    }

    // 6. Rename the edited template sheet to the output title.
    let title = if sheet.title.trim().is_empty() {
        sheet.template.as_str()
    } else {
        sheet.title.trim()
    };
    if title != sheet.template {
        ws.set_name(title);
    }
    Ok(())
}

/// Resolve a parameter to its data value. A `#name` is looked up first in the
/// record's **named** fields (JSON/YAML), then via the label's positional
/// schema; `#N` is positional. Missing bindings resolve to the empty string.
fn resolve_param(
    param: &Param,
    schema: Option<&Vec<String>>,
    fields: &[String],
    named: &std::collections::BTreeMap<String, String>,
) -> String {
    match param {
        Param::Indexed(n) => fields.get(n - 1).cloned().unwrap_or_default(),
        Param::Named(name) => {
            if let Some(v) = named.get(name) {
                return v.clone();
            }
            schema
                .and_then(|names| names.iter().position(|x| x == name))
                .and_then(|i| fields.get(i))
                .cloned()
                .unwrap_or_default()
        }
    }
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
        use std::collections::BTreeMap;
        let schema = vec!["date".to_string(), "receipt".to_string()];
        let fields = vec!["1/5/2009".to_string(), "22215".to_string()];
        let empty = BTreeMap::new();

        // Positional: by schema name and by index.
        assert_eq!(
            resolve_param(
                &Param::Named("receipt".into()),
                Some(&schema),
                &fields,
                &empty
            ),
            "22215"
        );
        assert_eq!(
            resolve_param(&Param::Indexed(1), Some(&schema), &fields, &empty),
            "1/5/2009"
        );

        // Named data takes precedence and needs no schema/positional fields.
        let named: BTreeMap<String, String> = [("receipt".to_string(), "99999".to_string())]
            .into_iter()
            .collect();
        assert_eq!(
            resolve_param(&Param::Named("receipt".into()), None, &[], &named),
            "99999"
        );

        // Unknown name or out-of-range index → empty.
        assert_eq!(
            resolve_param(&Param::Named("nope".into()), Some(&schema), &fields, &empty),
            ""
        );
        assert_eq!(resolve_param(&Param::Indexed(9), None, &fields, &empty), "");
    }
}
