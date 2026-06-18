//! The dynamic value type that flows through the engine.
//!
//! Data-file fields arrive as raw strings; the engine promotes anything that
//! parses as a number into [`Value::Number`] so that template expressions and
//! currency formatting can operate on it.

use std::fmt;

/// A single cell value: a number, some text, or nothing.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
    Empty,
}

impl Value {
    /// Promote a raw string field into a [`Value`], parsing numbers when possible.
    pub fn parse(raw: &str) -> Value {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            Value::Empty
        } else if let Ok(n) = trimmed.parse::<f64>() {
            Value::Number(n)
        } else {
            Value::Text(trimmed.to_string())
        }
    }

    /// The numeric view of this value, if it has one.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", format_number(*n)),
            Value::Text(s) => write!(f, "{s}"),
            Value::Empty => Ok(()),
        }
    }
}

/// Render a float without trailing zeros: `5.0 -> "5"`, `5.25 -> "5.25"`.
pub fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let s = format!("{n:.6}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Format a number as US currency with thousands separators: `1234.5 -> "$1,234.50"`.
pub fn format_currency(n: f64) -> String {
    let negative = n < 0.0;
    let cents = (n.abs() * 100.0).round() as u64;
    let whole = cents / 100;
    let frac = cents % 100;

    let digits = whole.to_string();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }

    let sign = if negative { "-" } else { "" };
    format!("{sign}${grouped}.{frac:02}")
}
