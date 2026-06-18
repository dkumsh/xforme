//! The template processor.
//!
//! Walks a parsed [`Sheet`] record by record, looks up the matching
//! [`Band`](crate::template::Band) in the [`Template`], resolves every cell's
//! placeholders and expressions against the record's fields plus the running
//! accumulators, and appends the result to a [`Document`].

use crate::data::Sheet;
use crate::document::{Cell, Document, Row};
use crate::expr::{self, Scope};
use crate::template::{Content, Template};
use crate::value::{Value, format_number};
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum EngineError {
    UnknownLabel(String),
    BadExpression { context: String, message: String },
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::UnknownLabel(l) => {
                write!(f, "no band defined for record label `{l}`")
            }
            EngineError::BadExpression { context, message } => {
                write!(f, "error evaluating `{context}`: {message}")
            }
        }
    }
}

impl std::error::Error for EngineError {}

/// Process a parsed sheet through a template into a renderable document.
pub fn process(template: &Template, sheet: &Sheet) -> Result<Document, EngineError> {
    let mut rows = Vec::new();
    // Running totals shared across bands (e.g. `subtotal`), available to every
    // subsequent record's scope.
    let mut accumulators: HashMap<String, f64> = HashMap::new();

    for record in &sheet.records {
        let band = template
            .bands
            .get(&record.label)
            .ok_or_else(|| EngineError::UnknownLabel(record.label.clone()))?;

        let scope = build_scope(template, &record.label, &record.fields, &accumulators);

        for row_spec in &band.rows {
            let mut cells = Vec::with_capacity(row_spec.cells.len());
            for cell_spec in &row_spec.cells {
                let value = resolve_content(&cell_spec.content, &scope)?;
                let mut style = cell_spec.style.clone();
                // Banded data rows tint every cell, not just the populated ones.
                if row_spec.banded && style.fill.is_none() {
                    style.fill = Some(BAND_FILL);
                }
                cells.push(Cell {
                    value,
                    style,
                    colspan: cell_spec.colspan,
                });
            }
            rows.push(Row { cells });
        }

        // Fold this band's contributions into the running totals.
        for (name, expr_src) in &band.accumulate {
            let amount = expr::eval(expr_src, &scope).map_err(|e| EngineError::BadExpression {
                context: expr_src.clone(),
                message: e.to_string(),
            })?;
            *accumulators.entry(name.clone()).or_insert(0.0) += amount;
        }
    }

    Ok(Document {
        title: sheet.title.clone(),
        columns: template.columns.clone(),
        rows,
    })
}

/// Light grey applied to alternating data rows.
const BAND_FILL: u32 = 0xF2F2F2;

/// Build the name -> value scope for one record: its named fields plus the
/// current accumulator values.
fn build_scope(
    template: &Template,
    label: &str,
    fields: &[String],
    accumulators: &HashMap<String, f64>,
) -> Scope {
    let mut scope = Scope::new();
    if let Some(names) = template.fields.get(label) {
        for (name, raw) in names.iter().zip(fields.iter()) {
            scope.insert(name.clone(), Value::parse(raw));
        }
    }
    for (name, value) in accumulators {
        scope.insert(name.clone(), Value::Number(*value));
    }
    scope
}

/// Resolve a single cell's content against the scope.
fn resolve_content(content: &Content, scope: &Scope) -> Result<Value, EngineError> {
    match content {
        Content::Empty => Ok(Value::Empty),
        Content::Literal(s) => Ok(Value::Text(s.clone())),
        Content::Text(template) => Ok(Value::Text(interpolate(template, scope))),
        Content::Expr(src) => {
            let n = expr::eval(src, scope).map_err(|e| EngineError::BadExpression {
                context: src.clone(),
                message: e.to_string(),
            })?;
            Ok(Value::Number(n))
        }
    }
}

/// Substitute `${...}` placeholders in `template`.
///
/// Each placeholder is first tried as an arithmetic expression (so
/// `${taxrate * 100}` works); failing that it is treated as a plain field
/// lookup, and an unknown name resolves to the empty string.
fn interpolate(template: &str, scope: &Scope) -> String {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$'
            && i + 1 < bytes.len()
            && bytes[i + 1] == b'{'
            && let Some(end) = template[i + 2..].find('}')
        {
            let inner = &template[i + 2..i + 2 + end];
            out.push_str(&resolve_placeholder(inner, scope));
            i = i + 2 + end + 1;
            continue;
        }
        let ch = template[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn resolve_placeholder(inner: &str, scope: &Scope) -> String {
    let trimmed = inner.trim();
    // Pure identifier: prefer the raw field value (keeps text fields intact).
    if let Some(value) = scope.get(trimmed) {
        return value.to_string();
    }
    // Otherwise try evaluating it as an expression.
    match expr::eval(trimmed, scope) {
        Ok(n) => format_number(n),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Scope;

    fn scope() -> Scope {
        let mut s = Scope::new();
        s.insert("taxrate".into(), Value::Number(0.0525));
        s.insert(
            "customer".into(),
            Value::Text("Jose Maria Fernandez".into()),
        );
        s
    }

    #[test]
    fn interpolates_fields_and_expressions() {
        assert_eq!(
            interpolate("Sold to: ${customer}", &scope()),
            "Sold to: Jose Maria Fernandez"
        );
        assert_eq!(
            interpolate("Tax (${taxrate * 100}%)", &scope()),
            "Tax (5.25%)"
        );
        assert_eq!(interpolate("${unknown}", &scope()), "");
    }
}
