//! Row-shifting for spreadsheet formulas.
//!
//! When a template row is replicated at a different output row, every *relative*
//! cell reference inside its formulas must move by the same row delta so the
//! arithmetic still points at the right cells. Absolute references (`$D$7`) and
//! the row parts marked absolute (`D$7`) are left untouched, and function names
//! such as `SUM` or `LOG10` are not mistaken for references.

/// Shift the row component of every relative cell reference in `formula` by
/// `delta`.
pub fn shift_rows(formula: &str, delta: i64) -> String {
    if delta == 0 {
        return formula.to_string();
    }

    let bytes = formula.as_bytes();
    let mut out = String::with_capacity(formula.len());
    let mut i = 0;

    while i < bytes.len() {
        // A cell reference cannot start immediately after an alphanumeric char
        // (that would be the tail of an identifier), so only attempt a match at
        // a boundary.
        let at_boundary = i == 0 || !is_ident_char(bytes[i - 1]);
        if at_boundary && let Some((consumed, shifted)) = try_shift_ref(&formula[i..], delta) {
            out.push_str(&shifted);
            i += consumed;
            continue;
        }
        let ch = formula[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Try to parse a cell reference at the start of `s`. On success returns the
/// number of bytes consumed and the (row-shifted) replacement text.
fn try_shift_ref(s: &str, delta: i64) -> Option<(usize, String)> {
    let bytes = s.as_bytes();
    let mut p = 0;

    let col_abs = bytes.first() == Some(&b'$');
    if col_abs {
        p += 1;
    }
    let letters_start = p;
    while p < bytes.len() && bytes[p].is_ascii_alphabetic() {
        p += 1;
    }
    let letters = &s[letters_start..p];
    if letters.is_empty() || letters.len() > 3 {
        return None; // not a column reference
    }

    let row_abs = bytes.get(p) == Some(&b'$');
    if row_abs {
        p += 1;
    }
    let digits_start = p;
    while p < bytes.len() && bytes[p].is_ascii_digit() {
        p += 1;
    }
    let digits = &s[digits_start..p];
    if digits.is_empty() {
        return None; // letters with no row number — not a cell ref
    }

    // A name immediately followed by `(` is a function call (e.g. `LOG10(`),
    // not a cell reference.
    if bytes.get(p) == Some(&b'(') {
        return None;
    }

    let row: i64 = digits.parse().ok()?;
    let new_row = if row_abs { row } else { (row + delta).max(1) };

    let mut replacement = String::new();
    if col_abs {
        replacement.push('$');
    }
    replacement.push_str(letters);
    if row_abs {
        replacement.push('$');
    }
    replacement.push_str(&new_row.to_string());

    Some((p, replacement))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_relative_refs() {
        assert_eq!(shift_rows("B7*D7", 5), "B12*D12");
        assert_eq!(shift_rows("E10+E11", 2), "E12+E13");
        assert_eq!(shift_rows("SUM(E7:E8)", 4), "SUM(E11:E12)");
    }

    #[test]
    fn leaves_absolute_refs() {
        assert_eq!(shift_rows("$D$7*A1", 5), "$D$7*A6");
        assert_eq!(shift_rows("D$7", 5), "D$7");
    }

    #[test]
    fn ignores_function_names_and_numbers() {
        assert_eq!(shift_rows("LOG10(A1)", 3), "LOG10(A4)");
        assert_eq!(shift_rows("A1*2+100", 1), "A2*2+100");
    }

    #[test]
    fn zero_delta_is_identity() {
        assert_eq!(shift_rows("SUM(E7:E8)", 0), "SUM(E7:E8)");
    }
}
