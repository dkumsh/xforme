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

/// A single data record: a label plus its positional fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub label: String,
    pub fields: Vec<String>,
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
            sheet.records.push(Record { label, fields });
        }
    }

    if let Some(sheet) = current.take() {
        sheets.push(sheet);
    }

    Ok(sheets)
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
                fields: vec![".0525".into()]
            }
        );
    }

    #[test]
    fn rejects_missing_header() {
        assert_eq!(parse("row1\t1\n"), Err(ParseError::MissingSheetHeader));
    }
}
