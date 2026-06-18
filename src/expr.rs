//! A tiny arithmetic expression evaluator used by template cells.
//!
//! Supports `+ - * /`, parentheses, unary minus, numeric literals and
//! identifiers resolved against a [`Scope`]. This is what lets a template say
//! `qty * price` for a line total or `subtotal + subtotal * taxrate` for a
//! grand total without baking those rules into the engine.

use crate::value::Value;
use std::collections::HashMap;

/// Name -> value bindings visible while evaluating an expression.
///
/// Holds both the current record's fields and the engine's running
/// accumulators (e.g. `subtotal`).
pub type Scope = HashMap<String, Value>;

#[derive(Debug, PartialEq)]
pub enum ExprError {
    Unexpected(String),
    UnknownIdent(String),
    NotANumber(String),
    Empty,
}

impl std::fmt::Display for ExprError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprError::Unexpected(s) => write!(f, "unexpected token near `{s}`"),
            ExprError::UnknownIdent(s) => write!(f, "unknown identifier `{s}`"),
            ExprError::NotANumber(s) => write!(f, "`{s}` is not a number"),
            ExprError::Empty => write!(f, "empty expression"),
        }
    }
}

impl std::error::Error for ExprError {}

/// Evaluate `input` to a number using bindings from `scope`.
pub fn eval(input: &str, scope: &Scope) -> Result<f64, ExprError> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(ExprError::Empty);
    }
    let mut parser = Parser {
        tokens,
        pos: 0,
        scope,
    };
    let value = parser.expression()?;
    if parser.pos != parser.tokens.len() {
        return Err(ExprError::Unexpected(format!(
            "{:?}",
            parser.tokens[parser.pos]
        )));
    }
    Ok(value)
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Result<Vec<Token>, ExprError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => i += 1,
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            _ if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let n = s.parse::<f64>().map_err(|_| ExprError::NotANumber(s))?;
                tokens.push(Token::Number(n));
            }
            _ if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Ident(s));
            }
            other => return Err(ExprError::Unexpected(other.to_string())),
        }
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    scope: &'a Scope,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// expression := term (('+' | '-') term)*
    fn expression(&mut self) -> Result<f64, ExprError> {
        let mut value = self.term()?;
        while let Some(op) = self.peek() {
            match op {
                Token::Plus => {
                    self.advance();
                    value += self.term()?;
                }
                Token::Minus => {
                    self.advance();
                    value -= self.term()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    /// term := factor (('*' | '/') factor)*
    fn term(&mut self) -> Result<f64, ExprError> {
        let mut value = self.factor()?;
        while let Some(op) = self.peek() {
            match op {
                Token::Star => {
                    self.advance();
                    value *= self.factor()?;
                }
                Token::Slash => {
                    self.advance();
                    value /= self.factor()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    /// factor := '-' factor | '(' expression ')' | number | ident
    fn factor(&mut self) -> Result<f64, ExprError> {
        match self.advance() {
            Some(Token::Minus) => Ok(-self.factor()?),
            Some(Token::Number(n)) => Ok(n),
            Some(Token::LParen) => {
                let v = self.expression()?;
                match self.advance() {
                    Some(Token::RParen) => Ok(v),
                    other => Err(ExprError::Unexpected(format!("{other:?}"))),
                }
            }
            Some(Token::Ident(name)) => match self.scope.get(&name) {
                Some(Value::Number(n)) => Ok(*n),
                Some(_) => Err(ExprError::NotANumber(name)),
                None => Err(ExprError::UnknownIdent(name)),
            },
            other => Err(ExprError::Unexpected(format!("{other:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> Scope {
        let mut s = Scope::new();
        s.insert("qty".into(), Value::Number(3.0));
        s.insert("price".into(), Value::Number(12.5));
        s.insert("subtotal".into(), Value::Number(100.0));
        s.insert("taxrate".into(), Value::Number(0.0525));
        s.insert("name".into(), Value::Text("Jose".into()));
        s
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(eval("2 + 3 * 4", &scope()).unwrap(), 14.0);
        assert_eq!(eval("(2 + 3) * 4", &scope()).unwrap(), 20.0);
        assert_eq!(eval("-5 + 2", &scope()).unwrap(), -3.0);
    }

    #[test]
    fn line_total_and_grand_total() {
        assert_eq!(eval("qty * price", &scope()).unwrap(), 37.5);
        assert_eq!(
            eval("subtotal + subtotal * taxrate", &scope()).unwrap(),
            105.25
        );
        assert_eq!(eval("taxrate * 100", &scope()).unwrap(), 5.25);
    }

    #[test]
    fn errors() {
        assert_eq!(
            eval("missing", &scope()),
            Err(ExprError::UnknownIdent("missing".into()))
        );
        assert_eq!(
            eval("name + 1", &scope()),
            Err(ExprError::NotANumber("name".into()))
        );
        assert_eq!(eval("", &scope()), Err(ExprError::Empty));
    }
}
