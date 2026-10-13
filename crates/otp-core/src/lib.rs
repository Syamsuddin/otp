// SPDX-License-Identifier: GPL-3.0-only
pub mod ast;
pub mod build;
pub mod lexer;
pub mod native;
pub mod parser;
use std::{fmt, path::Path};
#[derive(Debug, Clone)]
pub struct Error {
    pub line: usize,
    pub column: usize,
    pub message: String,
}
impl Error {
    pub fn new(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            column,
            message: message.into(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub range: bool,
    pub overflow: bool,
    pub debug: bool,
}
pub fn parse(source: &[u8]) -> Result<ast::Program, Error> {
    parser::Parser::new(lexer::lex(source)?).program()
}
pub fn compile(source: &[u8], target: native::Target, options: &Options) -> Result<String, Error> {
    let (source, options) = preprocess(source, options)?;
    let p = parse(&source)?;
    native::generate(&p, target, &options)
}
// This milestone accepts whole-program switches only, before the first token.
// Reject local switches explicitly, rather than silently changing their scope.
fn preprocess(src: &[u8], opts: &Options) -> Result<(Vec<u8>, Options), Error> {
    let mut opts = opts.clone();
    let mut out = src.to_vec();
    let mut i = 0;
    let mut line = 1;
    let mut code = false;
    while i < src.len() {
        if src[i].is_ascii_whitespace() {
            if src[i] == b'\n' {
                line += 1;
            }
            i += 1;
            continue;
        }
        if src[i..].starts_with(b"//") {
            while i < src.len() && src[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        let comment = if src[i] == b'{' {
            Some((1, b"}".as_slice()))
        } else if src[i..].starts_with(b"(*") {
            Some((2, b"*)".as_slice()))
        } else {
            None
        };
        if let Some((open, close)) = comment {
            let start = i;
            let start_line = line;
            i += open;
            let directive = src.get(i) == Some(&b'$');
            let content = i + usize::from(directive);
            while i < src.len() && !src[i..].starts_with(close) {
                if src[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            if i == src.len() {
                return Err(Error::new(start_line, 1, "Unterminated comment"));
            }
            if directive {
                if code {
                    return Err(Error::new(start_line,1,"Local compiler directives are not implemented; put supported switches before PROGRAM"));
                }
                let text = String::from_utf8_lossy(&src[content..i]).to_ascii_uppercase();
                for opt in text.split(',') {
                    match opt.trim() {
                        "R+" => opts.range = true,
                        "R-" => opts.range = false,
                        "Q+" => opts.overflow = true,
                        "Q-" => opts.overflow = false,
                        "D+" => opts.debug = true,
                        "D-" => opts.debug = false,
                        _ => {
                            return Err(Error::new(
                                start_line,
                                1,
                                format!("Directive '${opt}' is not implemented"),
                            ))
                        }
                    }
                }
                for ch in &mut out[start..i + close.len()] {
                    if *ch != b'\n' {
                        *ch = b' ';
                    }
                }
            }
            i += close.len();
            continue;
        }
        code = true;
        if src[i] == b'\'' {
            i += 1;
            while i < src.len() {
                if src[i] == b'\n' {
                    line += 1;
                }
                if src[i] == b'\'' {
                    i += 1;
                    if src.get(i) == Some(&b'\'') {
                        i += 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    Ok((out, opts))
}
pub fn compile_file(
    path: &Path,
    target: native::Target,
    options: &Options,
) -> Result<String, Error> {
    let src = std::fs::read(path).map_err(|e| Error::new(1, 1, e.to_string()))?;
    compile(&src, target, options)
}
