// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    ast::*,
    lexer::{Kind, Token},
    Error,
};
use otp_types::Type;
use std::collections::HashMap;
pub struct Parser {
    ts: Vec<Token>,
    i: usize,
    types: HashMap<String, Type>,
    ints: HashMap<String, i64>,
}
impl Parser {
    pub fn new(ts: Vec<Token>) -> Self {
        let mut types = HashMap::new();
        for (n, t) in [
            ("integer", Type::integer()),
            ("byte", Type::Int(8, false)),
            ("shortint", Type::Int(8, true)),
            ("word", Type::Int(16, false)),
            ("longint", Type::Int(32, true)),
            ("boolean", Type::Bool),
            ("char", Type::Char),
            ("string", Type::Str(255)),
            ("real", Type::Real),
            ("text", Type::Text),
        ] {
            types.insert(n.into(), t);
        }
        Self {
            ts,
            i: 0,
            types,
            ints: HashMap::new(),
        }
    }
    fn tok(&self) -> &Token {
        &self.ts[self.i]
    }
    fn is(&self, s: &str) -> bool {
        matches!(&self.tok().kind,Kind::Id(v)|Kind::Sym(v) if v==s)
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.is(s) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn err<T>(&self, m: impl Into<String>) -> Result<T, Error> {
        Err(Error::new(self.tok().line, self.tok().col, m))
    }
    fn need(&mut self, s: &str) -> Result<(), Error> {
        if self.eat(s) {
            Ok(())
        } else {
            self.err(format!("Expected '{s}'"))
        }
    }
    fn id(&mut self) -> Result<String, Error> {
        if let Kind::Id(n) = self.tok().kind.clone() {
            self.i += 1;
            Ok(n)
        } else {
            self.err("Identifier expected")
        }
    }
    pub fn program(&mut self) -> Result<Program, Error> {
        let name = if self.eat("program") {
            let n = self.id()?;
            if self.eat("(") {
                self.id()?;
                while self.eat(",") {
                    self.id()?;
                }
                self.need(")")?;
            }
            self.need(";")?;
            n
        } else {
            "program".into()
        };
        let mut uses = vec![];
        if self.eat("uses") {
            uses.push(self.id()?);
            while self.eat(",") {
                uses.push(self.id()?);
            }
            self.need(";")?;
        }
        let block = self.block()?;
        self.need(".")?;
        if !matches!(self.tok().kind, Kind::Eof) {
            return self.err("Unexpected data after program end");
        }
        Ok(Program { name, uses, block })
    }
    fn bound(&mut self) -> Result<i64, Error> {
        let neg = self.eat("-");
        self.eat("+");
        let n = match self.tok().kind.clone() {
            Kind::Int(v) => v,
            Kind::Id(n) => *self.ints.get(&n).ok_or_else(|| {
                Error::new(self.tok().line, self.tok().col, "Ordinal constant expected")
            })?,
            Kind::Str(v) if v.len() == 1 => v[0] as i64,
            _ => return self.err("Ordinal constant expected"),
        };
        self.i += 1;
        Ok(if neg { -n } else { n })
    }
    fn ty(&mut self) -> Result<Type, Error> {
        if self.eat("packed") {
            return self.err("packed layout is not implemented");
        }
        if self.eat("array") {
            self.need("[")?;
            let mut dims = vec![];
            loop {
                let l = self.bound()?;
                self.need("..")?;
                let h = self.bound()?;
                if h < l || h - l > 65535 {
                    return self.err("Invalid array bounds");
                }
                dims.push((l, h));
                if !self.eat(",") {
                    break;
                }
            }
            self.need("]")?;
            self.need("of")?;
            let mut t = self.ty()?;
            for (l, h) in dims.into_iter().rev() {
                t = Type::Array(l, h, Box::new(t));
            }
            return Ok(t);
        }
        if self.eat("record") {
            let mut fields = vec![];
            while !self.is("end") {
                let ns = self.names()?;
                self.need(":")?;
                let t = self.ty()?;
                for n in ns {
                    if fields.iter().any(|(x, _)| x == &n) {
                        return self.err("Duplicate record field");
                    }
                    fields.push((n, t.clone()));
                }
                if !self.eat(";") {
                    break;
                }
            }
            self.need("end")?;
            return Ok(Type::Record(fields));
        }
        let n = self.id()?;
        let t = self.types.get(&n).cloned().ok_or_else(|| {
            Error::new(
                self.tok().line,
                self.tok().col,
                format!("Unknown or unsupported type '{n}'"),
            )
        })?;
        if n == "string" && self.eat("[") {
            let len = self.bound()?;
            self.need("]")?;
            if !(0..=255).contains(&len) {
                return self.err("String capacity must be 0..255");
            }
            return Ok(Type::Str(len as usize));
        }
        Ok(t)
    }
    fn names(&mut self) -> Result<Vec<String>, Error> {
        let mut ns = vec![self.id()?];
        while self.eat(",") {
            ns.push(self.id()?);
        }
        Ok(ns)
    }
    fn declaration_start(&self) -> bool {
        matches!(&self.tok().kind,Kind::Id(n) if !["begin","const","var","type","procedure","function","label"].contains(&n.as_str()))
    }
    fn block(&mut self) -> Result<Block, Error> {
        let saved_types = self.types.clone();
        let saved_ints = self.ints.clone();
        let (mut constants, mut vars, mut routines) = (vec![], vec![], vec![]);
        loop {
            if self.eat("const") {
                while self.declaration_start() {
                    let n = self.id()?;
                    self.need("=")?;
                    let e = self.expr(0)?;
                    self.need(";")?;
                    if let Some(v) = const_int(&e, &self.ints) {
                        self.ints.insert(n.clone(), v);
                    }
                    constants.push((n, e));
                }
            } else if self.eat("type") {
                while self.declaration_start() {
                    let n = self.id()?;
                    self.need("=")?;
                    let t = self.ty()?;
                    self.need(";")?;
                    self.types.insert(n, t);
                }
            } else if self.eat("var") {
                while self.declaration_start() {
                    let ns = self.names()?;
                    self.need(":")?;
                    let t = self.ty()?;
                    self.need(";")?;
                    for n in ns {
                        vars.push((n, t.clone()));
                    }
                }
            } else if self.is("procedure") || self.is("function") {
                let line = self.tok().line;
                let f = self.eat("function");
                if !f {
                    self.need("procedure")?;
                }
                let name = self.id()?;
                let mut params = vec![];
                if self.eat("(") {
                    if !self.is(")") {
                        loop {
                            let by_ref = self.eat("var");
                            let ns = self.names()?;
                            self.need(":")?;
                            let ty = self.ty()?;
                            for name in ns {
                                params.push(Param {
                                    name,
                                    ty: ty.clone(),
                                    by_ref,
                                });
                            }
                            if !self.eat(";") {
                                break;
                            }
                        }
                    }
                    self.need(")")?;
                }
                let result = if f {
                    self.need(":")?;
                    self.ty()?
                } else {
                    Type::Void
                };
                self.need(";")?;
                if self.is("forward") || self.is("external") || self.is("assembler") {
                    return self.err("forward/external/assembler routine is not implemented");
                }
                let block = self.block()?;
                self.need(";")?;
                routines.push(Routine {
                    name,
                    params,
                    result,
                    block,
                    line,
                });
            } else {
                break;
            }
        }
        let body = self.statement()?;
        if !matches!(body.node, Node::Block(_)) {
            return self.err("BEGIN expected for block");
        }
        self.types = saved_types;
        self.ints = saved_ints;
        Ok(Block {
            constants,
            vars,
            routines,
            body,
        })
    }
    fn statements(&mut self, ends: &[&str]) -> Result<Vec<Stmt>, Error> {
        let mut v = vec![];
        while !ends.iter().any(|s| self.is(s)) {
            if matches!(self.tok().kind, Kind::Eof) {
                return self.err("Unexpected end of source");
            }
            v.push(self.statement()?);
            if !self.eat(";") {
                if !ends.iter().any(|s| self.is(s)) {
                    return self.err("Semicolon expected");
                }
                break;
            }
        }
        Ok(v)
    }
    fn statement(&mut self) -> Result<Stmt, Error> {
        let line = self.tok().line;
        let node = if self.eat("begin") {
            let v = self.statements(&["end"])?;
            self.need("end")?;
            Node::Block(v)
        } else if self.eat("if") {
            let e = self.expr(0)?;
            self.need("then")?;
            let y = Box::new(self.statement()?);
            let n = if self.eat("else") {
                Some(Box::new(self.statement()?))
            } else {
                None
            };
            Node::If(e, y, n)
        } else if self.eat("while") {
            let e = self.expr(0)?;
            self.need("do")?;
            Node::While(e, Box::new(self.statement()?))
        } else if self.eat("repeat") {
            let v = self.statements(&["until"])?;
            self.need("until")?;
            Node::Repeat(v, self.expr(0)?)
        } else if self.eat("for") {
            let v = Expr::Name(self.id()?);
            self.need(":=")?;
            let start = self.expr(0)?;
            let down = if self.eat("downto") {
                true
            } else {
                self.need("to")?;
                false
            };
            let end = self.expr(0)?;
            self.need("do")?;
            Node::For(v, start, end, down, Box::new(self.statement()?))
        } else if self.eat("case") {
            let e = self.expr(0)?;
            self.need("of")?;
            let mut arms = vec![];
            while !self.is("else") && !self.is("end") {
                let mut ranges = vec![];
                loop {
                    let lo = self.bound()?;
                    let hi = if self.eat("..") { self.bound()? } else { lo };
                    if hi < lo {
                        return self.err("Invalid case range");
                    }
                    for (old, _) in &arms {
                        for &(a, b) in old {
                            if lo <= b && hi >= a {
                                return self.err("Overlapping case labels");
                            }
                        }
                    }
                    if ranges.iter().any(|&(a, b)| lo <= b && hi >= a) {
                        return self.err("Overlapping case labels");
                    }
                    ranges.push((lo, hi));
                    if !self.eat(",") {
                        break;
                    }
                }
                self.need(":")?;
                let st = self.statement()?;
                arms.push((ranges, st));
                if !self.eat(";") {
                    break;
                }
            }
            let otherwise = if self.eat("else") {
                self.statements(&["end"])?
            } else {
                vec![]
            };
            self.need("end")?;
            Node::Case(e, arms, otherwise)
        } else if self.eat("break") {
            Node::Break
        } else if self.eat("continue") {
            Node::Continue
        } else if self.eat("exit") {
            Node::Exit
        } else if self.is("write") || self.is("writeln") {
            let nl = self.eat("writeln");
            if !nl {
                self.need("write")?;
            }
            let mut args = vec![];
            if self.eat("(") {
                if !self.is(")") {
                    loop {
                        let e = self.expr(0)?;
                        let w = if self.eat(":") {
                            Some(self.expr(0)?)
                        } else {
                            None
                        };
                        let d = if self.eat(":") {
                            Some(self.expr(0)?)
                        } else {
                            None
                        };
                        args.push((e, w, d));
                        if !self.eat(",") {
                            break;
                        }
                    }
                }
                self.need(")")?;
            }
            Node::Write(nl, args)
        } else if self.is(";") || self.is("end") || self.is("else") || self.is("until") {
            Node::Empty
        } else {
            let e = self.atom()?;
            if self.eat(":=") {
                Node::Assign(e, self.expr(0)?)
            } else {
                match e {
                    Expr::Name(n) => Node::Call(n, vec![]),
                    Expr::Call(n, args) => Node::Call(n, args),
                    _ => return self.err("Assignment expected"),
                }
            }
        };
        Ok(Stmt { line, node })
    }
    fn atom(&mut self) -> Result<Expr, Error> {
        let mut e = match self.tok().kind.clone() {
            Kind::Int(n) => {
                self.i += 1;
                Expr::Int(n)
            }
            Kind::Real(n) => {
                self.i += 1;
                Expr::Real(n)
            }
            Kind::Str(v) => {
                self.i += 1;
                Expr::Str(v)
            }
            Kind::Id(n) => {
                self.i += 1;
                if n == "true" || n == "false" {
                    Expr::Bool(n == "true")
                } else if self.eat("(") {
                    let mut args = vec![];
                    if !self.is(")") {
                        args.push(self.expr(0)?);
                        while self.eat(",") {
                            args.push(self.expr(0)?);
                        }
                    }
                    self.need(")")?;
                    Expr::Call(n, args)
                } else {
                    Expr::Name(n)
                }
            }
            Kind::Sym(n) if n == "(" => {
                self.i += 1;
                let e = self.expr(0)?;
                self.need(")")?;
                e
            }
            _ => return self.err("Expression expected"),
        };
        loop {
            if self.eat("[") {
                loop {
                    e = Expr::Index(Box::new(e), Box::new(self.expr(0)?));
                    if !self.eat(",") {
                        break;
                    }
                }
                self.need("]")?;
            } else if self.eat(".") {
                e = Expr::Field(Box::new(e), self.id()?);
            } else {
                break;
            }
        }
        Ok(e)
    }
    fn expr(&mut self, min: u8) -> Result<Expr, Error> {
        let mut lhs = if self.is("+") || self.is("-") || self.is("not") {
            let op = match self.tok().kind.clone() {
                Kind::Id(v) | Kind::Sym(v) => v,
                _ => unreachable!(),
            };
            self.i += 1;
            Expr::Unary(op, Box::new(self.expr(40)?))
        } else {
            self.atom()?
        };
        loop {
            let op = match &self.tok().kind {
                Kind::Id(s) | Kind::Sym(s) => s.clone(),
                _ => break,
            };
            let p = match op.as_str() {
                "=" | "<>" | "<" | ">" | "<=" | ">=" => 10,
                "+" | "-" | "or" | "xor" => 20,
                "*" | "/" | "div" | "mod" | "and" | "shl" | "shr" => 30,
                _ => break,
            };
            if p < min {
                break;
            }
            self.i += 1;
            let rhs = self.expr(p + 1)?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }
}
fn const_int(e: &Expr, map: &HashMap<String, i64>) -> Option<i64> {
    match e {
        Expr::Int(n) => Some(*n),
        Expr::Name(n) => map.get(n).copied(),
        Expr::Unary(op, x) if op == "-" => Some(-const_int(x, map)?),
        _ => None,
    }
}
