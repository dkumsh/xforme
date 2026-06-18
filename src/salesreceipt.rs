//! The canonical **Sales Receipt** example, rebuilt on the generic engine.
//!
//! This mirrors the original templateIt demo
//! (<https://templateit.sourceforge.net/SalesReceipt.html>): the same
//! `header` / `row1` / `row2` / `footer` record labels drive a four-column
//! receipt — Qty, Description, Unit Price, Amount — with a subtotal, sales tax
//! computed from the footer's rate, and a grand total.

use crate::template::{Align, Band, CellSpec, ColumnSpec, Content, RowSpec, Style, Template};
use std::collections::HashMap;

/// Header row background.
const HEADER_FILL: u32 = 0xD9E1F2;

/// Build the Sales Receipt template.
pub fn template() -> Template {
    Template {
        name: "SalesReceipt".to_string(),
        columns: columns(),
        fields: fields(),
        bands: bands(),
    }
}

fn columns() -> Vec<ColumnSpec> {
    vec![
        ColumnSpec {
            xlsx_width: 8.0,
            pdf_width: 18.0,
        }, // Qty
        ColumnSpec {
            xlsx_width: 44.0,
            pdf_width: 96.0,
        }, // Description
        ColumnSpec {
            xlsx_width: 14.0,
            pdf_width: 28.0,
        }, // Unit Price
        ColumnSpec {
            xlsx_width: 14.0,
            pdf_width: 28.0,
        }, // Amount
    ]
}

fn fields() -> HashMap<String, Vec<String>> {
    let mut f = HashMap::new();
    f.insert(
        "header".to_string(),
        vec![
            "date".into(),
            "receipt".into(),
            "customer".into(),
            "address".into(),
        ],
    );
    let line_fields = vec![
        "seq".to_string(),
        "qty".into(),
        "desc".into(),
        "price".into(),
    ];
    f.insert("row1".to_string(), line_fields.clone());
    f.insert("row2".to_string(), line_fields);
    f.insert("footer".to_string(), vec!["taxrate".into()]);
    f
}

fn bands() -> HashMap<String, Band> {
    let mut b = HashMap::new();
    b.insert("header".to_string(), header_band());
    b.insert("row1".to_string(), line_band(false));
    b.insert("row2".to_string(), line_band(true));
    b.insert("footer".to_string(), footer_band());
    b
}

fn header_band() -> Band {
    Band {
        rows: vec![
            // Title spanning the full width.
            RowSpec::new(vec![
                CellSpec::literal("SALES RECEIPT")
                    .span(4)
                    .style(Style::new().bold().align(Align::Center).size(18.0)),
            ]),
            // Receipt number (left) and date (right).
            RowSpec::new(vec![
                CellSpec::text("Receipt #: ${receipt}")
                    .span(2)
                    .style(Style::new().bold()),
                CellSpec::text("Date: ${date}")
                    .span(2)
                    .style(Style::new().align(Align::Right)),
            ]),
            // Customer and address.
            RowSpec::new(vec![CellSpec::text("Sold To: ${customer}").span(4)]),
            RowSpec::new(vec![CellSpec::text("${address}").span(4)]),
            // Spacer.
            RowSpec::new(vec![CellSpec::empty().span(4)]),
            // Column headers.
            RowSpec::new(vec![
                CellSpec::literal("Qty").style(
                    Style::new()
                        .bold()
                        .align(Align::Right)
                        .fill(HEADER_FILL)
                        .border_top(),
                ),
                CellSpec::literal("Description").style(
                    Style::new()
                        .bold()
                        .align(Align::Left)
                        .fill(HEADER_FILL)
                        .border_top(),
                ),
                CellSpec::literal("Unit Price").style(
                    Style::new()
                        .bold()
                        .align(Align::Right)
                        .fill(HEADER_FILL)
                        .border_top(),
                ),
                CellSpec::literal("Amount").style(
                    Style::new()
                        .bold()
                        .align(Align::Right)
                        .fill(HEADER_FILL)
                        .border_top(),
                ),
            ]),
        ],
        accumulate: Vec::new(),
    }
}

/// A line-item band. `shaded` toggles the banded background used by `row2`.
fn line_band(shaded: bool) -> Band {
    let mut row = RowSpec::new(vec![
        CellSpec::expr("qty").style(Style::new().align(Align::Right)),
        CellSpec {
            content: Content::Text("${desc}".into()),
            ..CellSpec::empty()
        },
        CellSpec::expr("price").style(Style::new().align(Align::Right).currency()),
        CellSpec::expr("qty * price").style(Style::new().align(Align::Right).currency()),
    ]);
    row.banded = shaded;
    Band {
        rows: vec![row],
        accumulate: vec![("subtotal".to_string(), "qty * price".to_string())],
    }
}

fn footer_band() -> Band {
    Band {
        rows: vec![
            RowSpec::new(vec![CellSpec::empty().span(4)]),
            // Subtotal.
            RowSpec::new(vec![
                CellSpec::empty().span(2),
                CellSpec::literal("Subtotal:").style(Style::new().bold().align(Align::Right)),
                CellSpec::expr("subtotal").style(Style::new().align(Align::Right).currency()),
            ]),
            // Sales tax, label showing the rate as a percentage.
            RowSpec::new(vec![
                CellSpec::empty().span(2),
                CellSpec::text("Tax (${taxrate * 100}%):").style(Style::new().align(Align::Right)),
                CellSpec::expr("subtotal * taxrate")
                    .style(Style::new().align(Align::Right).currency()),
            ]),
            // Grand total, separated by a top rule.
            RowSpec::new(vec![
                CellSpec::empty().span(2),
                CellSpec::literal("TOTAL:")
                    .style(Style::new().bold().align(Align::Right).border_top()),
                CellSpec::expr("subtotal + subtotal * taxrate").style(
                    Style::new()
                        .bold()
                        .align(Align::Right)
                        .currency()
                        .border_top(),
                ),
            ]),
        ],
        accumulate: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{data, engine};

    const SAMPLE: &str = include_str!("../data/sales_receipt.txt");

    #[test]
    fn computes_totals_from_sample_data() {
        let sheets = data::parse(SAMPLE).expect("parse");
        let doc = engine::process(&template(), &sheets[0]).expect("process");

        // Find every currency-formatted Amount cell in the data rows by scanning
        // for the grand total: subtotal = 53 + 14 = 67, tax = 67 * 0.0525 = 3.5175.
        let numbers: Vec<f64> = doc
            .rows
            .iter()
            .flat_map(|r| r.cells.iter())
            .filter_map(|c| c.value.as_number())
            .collect();

        // Grand total should appear: 67 + 3.5175 = 70.5175.
        assert!(
            numbers.iter().any(|n| (n - 70.5175).abs() < 1e-6),
            "totals: {numbers:?}"
        );
        assert!(
            numbers.iter().any(|n| (n - 67.0).abs() < 1e-6),
            "subtotal present"
        );
    }
}
