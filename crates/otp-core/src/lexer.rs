// SPDX-License-Identifier: GPL-3.0-only
use crate::Error;
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Id(String),
    Int(i64),
    Real(f64),
    Str(Vec<u8>),
    Sym(String),
    Eof,
}
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: Kind,
    pub line: usize,
    pub col: usize,
}
pub fn lex(src: &[u8]) -> Result<Vec<Token>, Error> {
    let (mut i, mut line, mut col) = (0, 1, 1);
    let mut out = vec![];
    macro_rules! bump {
        () => {{
            if src[i] == b'\n' {
                line += 1;
                col = 1
            } else {
                col += 1
            }
            i += 1;
        }};
    }
    while i < src.len() {
        let b = src[i];
        if b.is_ascii_whitespace() {
            bump!();
            continue;
        }
        if b == b'{' || (b == b'(' && src.get(i + 1) == Some(&b'*')) {
            let braces = b == b'{';
            let (l, c) = (line, col);
            bump!();
            if !braces {
                bump!();
            }
            let mut closed = false;
            while i < src.len() {
                if (braces && src[i] == b'}')
                    || (!braces && src[i] == b'*' && src.get(i + 1) == Some(&b')'))
                {
                    bump!();
                    if !braces {
                        bump!();
                    }
                    closed = true;
                    break;
                }
                bump!();
            }
            if !closed {
                return Err(Error::new(l, c, "Unterminated comment"));
            }
            continue;
        }
        if b == b'/' && src.get(i + 1) == Some(&b'/') {
            while i < src.len() && src[i] != b'\n' {
                bump!();
            }
            continue;
        }
        let (l, c, start) = (line, col, i);
        let kind = if b.is_ascii_alphabetic() || b == b'_' {
            bump!();
            while i < src.len() && (src[i].is_ascii_alphanumeric() || src[i] == b'_') {
                bump!();
            }
            Kind::Id(String::from_utf8_lossy(&src[start..i]).to_ascii_lowercase())
        } else if b.is_ascii_digit() || b == b'$' {
            let hex = b == b'$';
            bump!();
            while i < src.len()
                && if hex {
                    src[i].is_ascii_hexdigit()
                } else {
                    src[i].is_ascii_digit()
                }
            {
                bump!();
            }
            let mut real = false;
            if !hex && src.get(i) == Some(&b'.') && src.get(i + 1) != Some(&b'.') {
                real = true;
                bump!();
                while i < src.len() && src[i].is_ascii_digit() {
                    bump!();
                }
            }
            if !hex && matches!(src.get(i), Some(b'e' | b'E')) {
                real = true;
                bump!();
                if matches!(src.get(i), Some(b'+' | b'-')) {
                    bump!();
                }
                while i < src.len() && src[i].is_ascii_digit() {
                    bump!();
                }
            }
            let s = String::from_utf8_lossy(&src[start..i]);
            if real {
                Kind::Real(
                    s.parse()
                        .map_err(|_| Error::new(l, c, "Invalid real literal"))?,
                )
            } else {
                Kind::Int(
                    if hex {
                        i64::from_str_radix(&s[1..], 16)
                    } else {
                        s.parse()
                    }
                    .map_err(|_| Error::new(l, c, "Invalid integer literal"))?,
                )
            }
        } else if b == b'\'' || b == b'#' {
            let mut val = vec![];
            while i < src.len() && (src[i] == b'\'' || src[i] == b'#') {
                if src[i] == b'#' {
                    bump!();
                    let st = i;
                    while i < src.len() && src[i].is_ascii_digit() {
                        bump!();
                    }
                    let n = String::from_utf8_lossy(&src[st..i])
                        .parse::<u8>()
                        .map_err(|_| Error::new(l, c, "Character code outside 0..255"))?;
                    val.push(n);
                } else {
                    bump!();
                    let mut closed = false;
                    while i < src.len() {
                        if src[i] == b'\'' {
                            bump!();
                            if src.get(i) == Some(&b'\'') {
                                val.push(b'\'');
                                bump!();
                            } else {
                                closed = true;
                                break;
                            }
                        } else {
                            if src[i] == b'\n' {
                                return Err(Error::new(l, c, "String crosses line"));
                            }
                            val.push(src[i]);
                            bump!();
                        }
                    }
                    if !closed {
                        return Err(Error::new(l, c, "Unterminated string"));
                    }
                }
            }
            if val.len() > 255 {
                return Err(Error::new(l, c, "String exceeds 255 bytes"));
            }
            Kind::Str(val)
        } else {
            bump!();
            let mut s = (b as char).to_string();
            if i < src.len() {
                let pair = format!("{}{}", b as char, src[i] as char);
                if [":=", "<=", ">=", "<>", ".."].contains(&pair.as_str()) {
                    s = pair;
                    bump!();
                }
            }
            if ![
                "+", "-", "*", "/", "=", "<", ">", "(", ")", "[", "]", ",", ";", ":", ".", ":=",
                "<=", ">=", "<>", "..",
            ]
            .contains(&s.as_str())
            {
                return Err(Error::new(l, c, format!("Unsupported character {s}")));
            }
            Kind::Sym(s)
        };
        out.push(Token {
            kind,
            line: l,
            col: c,
        });
    }
    out.push(Token {
        kind: Kind::Eof,
        line,
        col,
    });
    Ok(out)
}
