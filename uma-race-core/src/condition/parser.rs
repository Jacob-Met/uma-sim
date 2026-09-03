//! Tokenizer + recursive-descent parser for skill condition DSL.
//! Grammar (community / GameTora): `term ((&|@) term)*` where
//! `term = ident (op number)?` and `op ∈ {==,!=,>=,<=,>,<}`.
//! Bare `ident` means `ident==1`.

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Eq,
    Ne,
    Ge,
    Le,
    Gt,
    Lt,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Atom {
    pub name: String,
    pub op: Op,
    pub value: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Atom(Atom),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>), // `@` in the DSL
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub at: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at {}: {}", self.at, self.message)
    }
}

impl std::error::Error for ParseError {}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ident(String),
    Number(i64),
    Op(Op),
    And,
    Or,
}

fn tokenize(input: &str) -> Result<Vec<Tok>, ParseError> {
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'&' => {
                out.push(Tok::And);
                i += 1;
            }
            b'@' => {
                out.push(Tok::Or);
                i += 1;
            }
            b'=' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                out.push(Tok::Op(Op::Eq));
                i += 2;
            }
            b'!' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                out.push(Tok::Op(Op::Ne));
                i += 2;
            }
            b'>' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                out.push(Tok::Op(Op::Ge));
                i += 2;
            }
            b'<' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                out.push(Tok::Op(Op::Le));
                i += 2;
            }
            b'>' => {
                out.push(Tok::Op(Op::Gt));
                i += 1;
            }
            b'<' => {
                out.push(Tok::Op(Op::Lt));
                i += 1;
            }
            b'0'..=b'9' | b'-' => {
                let start = i;
                if bytes[i] == b'-' {
                    i += 1;
                }
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let s = std::str::from_utf8(&bytes[start..i]).unwrap();
                let n: i64 = s.parse().map_err(|_| ParseError {
                    message: format!("bad number {s}"),
                    at: start,
                })?;
                out.push(Tok::Number(n));
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let start = i;
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_')
                {
                    i += 1;
                }
                out.push(Tok::Ident(
                    std::str::from_utf8(&bytes[start..i]).unwrap().to_string(),
                ));
            }
            _ => {
                return Err(ParseError {
                    message: format!("unexpected byte {:?}", bytes[i] as char),
                    at: i,
                });
            }
        }
    }
    Ok(out)
}

struct Parser<'a> {
    toks: &'a [Tok],
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a Tok> {
        self.toks.get(self.i)
    }
    fn bump(&mut self) -> Option<&'a Tok> {
        let t = self.toks.get(self.i);
        if t.is_some() {
            self.i += 1;
        }
        t
    }

    fn atom(&mut self) -> Result<Expr, ParseError> {
        let name = match self.bump() {
            Some(Tok::Ident(s)) => s.clone(),
            other => {
                return Err(ParseError {
                    message: format!("expected ident, got {other:?}"),
                    at: self.i,
                });
            }
        };
        let (op, value) = match self.peek() {
            Some(Tok::Op(op)) => {
                let op = op.clone();
                self.bump();
                match self.bump() {
                    Some(Tok::Number(n)) => (op, *n),
                    _ => {
                        return Err(ParseError {
                            message: "expected number after op".into(),
                            at: self.i,
                        });
                    }
                }
            }
            _ => (Op::Eq, 1), // bare ident ⇒ ==1
        };
        Ok(Expr::Atom(Atom { name, op, value }))
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.atom()?;
        while matches!(self.peek(), Some(Tok::And)) {
            self.bump();
            let right = self.atom()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        // `@` (OR) binds looser than `&` (AND): `a&b@c&d` ⇒ `(a&b)@(c&d)`
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Some(Tok::Or)) {
            self.bump();
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }
}

/// Parse a condition or precondition string. Empty string → `None`.
pub fn parse_condition(input: &str) -> Result<Option<Expr>, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let toks = tokenize(trimmed)?;
    let mut p = Parser { toks: &toks, i: 0 };
    let expr = p.parse_expr()?;
    if p.i != toks.len() {
        return Err(ParseError {
            message: format!("trailing tokens starting at {}", p.i),
            at: p.i,
        });
    }
    Ok(Some(expr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn parses_basic_and_or() {
        let e = parse_condition("phase>=2&order<=5@is_finalcorner").unwrap().unwrap();
        match e {
            Expr::Or(a, b) => {
                assert!(matches!(*a, Expr::And(_, _)));
                assert!(matches!(
                    *b,
                    Expr::Atom(Atom {
                        op: Op::Eq,
                        value: 1,
                        ..
                    })
                ));
            }
            _ => panic!("expected Or"),
        }
    }

    #[test]
    fn or_binds_looser_than_and() {
        let e = parse_condition("a==1&b==2@c==3&d==4").unwrap().unwrap();
        match e {
            Expr::Or(left, right) => {
                assert!(matches!(*left, Expr::And(_, _)));
                assert!(matches!(*right, Expr::And(_, _)));
            }
            _ => panic!("expected top-level Or"),
        }
    }

    #[test]
    fn all_skill_condition_strings_parse() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../knowledge/canonical/by_kind/skill.json");
        let skills: Vec<serde_json::Value> =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let mut uniq = HashSet::new();
        let mut failed = Vec::new();
        for s in &skills {
            for g in s["payload"]["condition_groups"].as_array().into_iter().flatten() {
                for key in ["condition", "precondition"] {
                    if let Some(c) = g[key].as_str() {
                        if uniq.insert(c.to_string()) {
                            if let Err(e) = parse_condition(c) {
                                failed.push(format!("{key}={c:?}: {e}"));
                            }
                        }
                    }
                }
            }
        }
        assert!(
            failed.is_empty(),
            "{} parse failures (of {} unique). First: {}",
            failed.len(),
            uniq.len(),
            failed.first().unwrap_or(&String::new())
        );
        assert!(uniq.len() > 100, "expected many unique conditions, got {}", uniq.len());
    }
}
