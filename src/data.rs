//! Parser for the templateIt data-stream format.
//!
//! The format is tab-delimited and line-oriented, matching the original Sales
//! Receipt example:
//!
//! ```text
//! #sheet  SalesReceipt    Sales Receipt
//! header  1/5/2009    22215   Jose Maria Fernandez    1010 Broadway...
//! row1    1   1   Introduction to Algebra 53.0
//! row2    2   1   Introduction to Algebra Solutions Manual    14.0
//! footer  .0525
//! ##end
//! ```
//!
//! * `#sheet <template> <title...>` opens a sheet; the rest of the line is the
//!   sheet title.
//! * `##end` closes the current sheet.
//! * Any other line is a record: the first field is its label, the remaining
//!   tab-separated fields are its data.
//!
//! JSON and YAML inputs (Cargo features `json` / `yaml`) are also supported via
//! [`parse_json`] / [`parse_yaml`]. They use the same record-stream model, but
//! let each record carry **named** fields resolved directly by `#name`.

use std::collections::BTreeMap;

/// A single data record: a label plus its data fields.
///
/// Fields may be **positional** (`fields` — the tab-delimited form and `#N`
/// parameters) and/or **named** (`named` — from JSON/YAML, resolved by `#name`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Record {
    pub label: String,
    pub fields: Vec<String>,
    pub named: BTreeMap<String, String>,
}

/// One parsed sheet: which template to use, the title, and its records.
#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    pub template: String,
    pub title: String,
    pub records: Vec<Record>,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    MissingSheetHeader,
    MalformedSheet(usize),
    UnexpectedEnd(usize),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::MissingSheetHeader => write!(f, "data does not start with a `#sheet` line"),
            ParseError::MalformedSheet(n) => write!(f, "malformed `#sheet` line at line {n}"),
            ParseError::UnexpectedEnd(n) => write!(f, "`##end` without an open sheet at line {n}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a data stream into its constituent sheets.
pub fn parse(input: &str) -> Result<Vec<Sheet>, ParseError> {
    let mut sheets = Vec::new();
    let mut current: Option<Sheet> = None;

    for (idx, raw_line) in input.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim_end_matches(['\r', '\n']);
        if line.trim().is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("#sheet") {
            let mut parts = rest.trim_start().splitn(2, '\t');
            let template = parts.next().map(str::trim).unwrap_or("");
            if template.is_empty() {
                return Err(ParseError::MalformedSheet(line_no));
            }
            let title = parts.next().map(str::trim).unwrap_or("").to_string();
            // Flush any sheet left unterminated before starting a new one.
            if let Some(sheet) = current.take() {
                sheets.push(sheet);
            }
            current = Some(Sheet {
                template: template.to_string(),
                title,
                records: Vec::new(),
            });
        } else if line.trim() == "##end" {
            match current.take() {
                Some(sheet) => sheets.push(sheet),
                None => return Err(ParseError::UnexpectedEnd(line_no)),
            }
        } else {
            let sheet = current.as_mut().ok_or(ParseError::MissingSheetHeader)?;
            let mut fields: Vec<String> = line.split('\t').map(|s| s.trim().to_string()).collect();
            let label = fields.remove(0);
            sheet.records.push(Record {
                label,
                fields,
                ..Default::default()
            });
        }
    }

    if let Some(sheet) = current.take() {
        sheets.push(sheet);
    }

    Ok(sheets)
}

/// Parse a JSON document into sheets.
///
/// The document is a sheet object (or an array of them):
///
/// ```json
/// { "template": "SalesReceipt", "title": "Sales Receipt", "records": [
///     { "label": "header", "date": "1/5/2009", "receipt": "22215" },
///     { "label": "row1", "qty": 1, "desc": "Algebra", "price": 53.0 }
/// ] }
/// ```
///
/// Each record's keys other than `label` (and an optional positional `fields`
/// array) become its [`Record::named`] fields, resolved by `#name`.
#[cfg(feature = "json")]
pub fn parse_json(input: &str) -> Result<Vec<Sheet>, Box<dyn std::error::Error>> {
    sheets_from_value(serde_json::from_str(input)?)
}

/// Parse a YAML document into sheets — the YAML analogue of [`parse_json`],
/// with the same shape.
#[cfg(feature = "yaml")]
pub fn parse_yaml(input: &str) -> Result<Vec<Sheet>, Box<dyn std::error::Error>> {
    sheets_from_value(serde_norway::from_str(input)?)
}

// Shared JSON/YAML handling: both formats are deserialized into a
// `serde_json::Value` and walked here, so there's one code path.
#[cfg(any(feature = "json", feature = "yaml"))]
fn sheets_from_value(v: serde_json::Value) -> Result<Vec<Sheet>, Box<dyn std::error::Error>> {
    match v {
        serde_json::Value::Array(arr) => arr.into_iter().map(sheet_from_value).collect(),
        obj @ serde_json::Value::Object(_) => Ok(vec![sheet_from_value(obj)?]),
        _ => Err("expected a sheet object or an array of sheets".into()),
    }
}

#[cfg(any(feature = "json", feature = "yaml"))]
fn sheet_from_value(v: serde_json::Value) -> Result<Sheet, Box<dyn std::error::Error>> {
    use serde_json::Value;
    let obj = v.as_object().ok_or("sheet must be an object")?;
    let template = obj
        .get("template")
        .and_then(Value::as_str)
        .ok_or("sheet is missing a string `template`")?
        .to_string();
    let title = obj
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let records_v = obj
        .get("records")
        .and_then(Value::as_array)
        .ok_or("sheet is missing a `records` array")?;

    let mut records = Vec::with_capacity(records_v.len());
    for rv in records_v {
        let robj = rv.as_object().ok_or("each record must be an object")?;
        let label = robj
            .get("label")
            .and_then(Value::as_str)
            .ok_or("a record is missing a string `label`")?
            .to_string();

        // Optional positional `fields` array (for `#N` parameters / TSV parity).
        let mut fields = Vec::new();
        if let Some(Value::Array(arr)) = robj.get("fields") {
            fields.extend(arr.iter().filter_map(value_to_field));
        }
        // Every other key is a named field, resolved by `#name`.
        let mut named = BTreeMap::new();
        for (k, val) in robj {
            if k == "label" || k == "fields" {
                continue;
            }
            if let Some(s) = value_to_field(val) {
                named.insert(k.clone(), s);
            }
        }
        records.push(Record {
            label,
            fields,
            named,
        });
    }
    Ok(Sheet {
        template,
        title,
        records,
    })
}

/// Stringify a scalar JSON value for use as a field (non-scalars are skipped).
#[cfg(any(feature = "json", feature = "yaml"))]
fn value_to_field(v: &serde_json::Value) -> Option<String> {
    use serde_json::Value;
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "#sheet\tSalesReceipt\tSales Receipt\n\
                          header\t1/5/2009\t22215\tJose Maria Fernandez\t1010 Broadway\n\
                          row1\t1\t1\tIntroduction to Algebra\t53.0\n\
                          footer\t.0525\n\
                          ##end\n";

    #[test]
    fn parses_sheet_and_records() {
        let sheets = parse(SAMPLE).unwrap();
        assert_eq!(sheets.len(), 1);
        let s = &sheets[0];
        assert_eq!(s.template, "SalesReceipt");
        assert_eq!(s.title, "Sales Receipt");
        assert_eq!(s.records.len(), 3);
        assert_eq!(s.records[0].label, "header");
        assert_eq!(s.records[0].fields[2], "Jose Maria Fernandez");
        assert_eq!(
            s.records[1].fields,
            vec!["1", "1", "Introduction to Algebra", "53.0"]
        );
        assert_eq!(
            s.records[2],
            Record {
                label: "footer".into(),
                fields: vec![".0525".into()],
                ..Default::default()
            }
        );
    }

    #[test]
    fn rejects_missing_header() {
        assert_eq!(parse("row1\t1\n"), Err(ParseError::MissingSheetHeader));
    }

    #[cfg(feature = "json")]
    #[test]
    fn parses_json_named_records() {
        let src = r#"{
            "template": "SalesReceipt", "title": "Sales Receipt",
            "records": [
                { "label": "header", "date": "1/5/2009", "receipt": "22215" },
                { "label": "row1", "qty": 2, "desc": "Widget", "price": 9.99 }
            ]
        }"#;
        let sheets = parse_json(src).unwrap();
        assert_eq!(sheets.len(), 1);
        let s = &sheets[0];
        assert_eq!(s.template, "SalesReceipt");
        assert_eq!(s.title, "Sales Receipt");
        assert_eq!(s.records[0].named["receipt"], "22215");
        assert_eq!(s.records[1].named["qty"], "2"); // integer
        assert_eq!(s.records[1].named["price"], "9.99"); // float
        assert_eq!(s.records[1].label, "row1");
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn parses_yaml_named_records() {
        let src = "template: SalesReceipt\n\
                   title: Sales Receipt\n\
                   records:\n\
                   \x20 - label: row1\n\
                   \x20   qty: 2\n\
                   \x20   desc: Widget\n\
                   \x20   price: 9.99\n";
        let sheets = parse_yaml(src).unwrap();
        let r = &sheets[0].records[0];
        assert_eq!(r.label, "row1");
        assert_eq!(r.named["qty"], "2");
        assert_eq!(r.named["price"], "9.99");
        assert_eq!(r.named["desc"], "Widget");
    }
}
