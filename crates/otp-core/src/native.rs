// SPDX-License-Identifier: GPL-3.0-only
use crate::{ast::*, Error, Options};
use otp_types::Type;
use std::collections::HashMap;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    LinuxX64,
    WindowsX64,
    MacX64,
    MacArm64,
}
impl Target {
    pub fn host() -> Result<Self, String> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => Ok(Self::LinuxX64),
            ("windows", "x86_64") => Ok(Self::WindowsX64),
            ("macos", "x86_64") => Ok(Self::MacX64),
            ("macos", "aarch64") => Ok(Self::MacArm64),
            _ => Err("Unsupported host".into()),
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "linux-x64" => Some(Self::LinuxX64),
            "windows-x64" => Some(Self::WindowsX64),
            "macos-x64" => Some(Self::MacX64),
            "macos-arm64" => Some(Self::MacArm64),
            _ => None,
        }
    }
    fn apple(self) -> bool {
        matches!(self, Self::MacX64 | Self::MacArm64)
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::LinuxX64 => "linux-x64",
            Self::WindowsX64 => "windows-x64",
            Self::MacX64 => "macos-x64",
            Self::MacArm64 => "macos-arm64",
        }
    }
}
#[derive(Clone)]
enum Sym {
    Var(usize, Type),
    Const(Expr),
    Routine(usize, Vec<Param>, Type),
}
#[derive(Clone)]
struct Scope {
    parent: Option<usize>,
    syms: HashMap<String, Sym>,
    slots: usize,
    decls: Vec<(usize, String, Type)>,
}
struct Gen {
    target: Target,
    opts: Options,
    scopes: Vec<Scope>,
    current: usize,
    code: String,
    data: Vec<(String, Vec<u8>)>,
    serial: usize,
    loops: Vec<(String, String)>,
    ret: String,
    line: usize,
    constants: Vec<(usize, String)>,
    type_cache: HashMap<(usize, usize), Type>,
}
#[derive(Clone)]
enum Arg {
    N(i64),
    Label(String),
}
impl Gen {
    fn label(&mut self) -> String {
        self.serial += 1;
        format!("L_otp_{}", self.serial)
    }
    fn literal(&mut self, s: &[u8]) -> Arg {
        let name = self.label();
        self.data.push((name.clone(), s.to_vec()));
        Arg::Label(name)
    }
    fn symbol(&self, s: &str) -> String {
        if self.target.apple() {
            format!("_{s}")
        } else {
            s.into()
        }
    }
    fn line(&mut self, s: impl AsRef<str>) {
        self.code += s.as_ref();
        self.code.push('\n');
    }
    fn call(&mut self, name: &str, args: &[Arg]) {
        for (i, arg) in args.iter().enumerate() {
            if self.target == Target::MacArm64 {
                match arg {
                    Arg::N(n) => {
                        let v = *n as u64;
                        self.line(format!("movz x{i}, #{}", v & 65535));
                        for j in 1..4 {
                            let w = (v >> (j * 16)) & 65535;
                            if w != 0 {
                                self.line(format!("movk x{i}, #{w}, lsl #{}", j * 16));
                            }
                        }
                    }
                    Arg::Label(s) => {
                        self.line(format!("adrp x{i}, {s}@PAGE\nadd x{i}, x{i}, {s}@PAGEOFF"));
                    }
                }
            } else {
                let regs = if self.target == Target::WindowsX64 {
                    vec!["%rcx", "%rdx", "%r8", "%r9"]
                } else {
                    vec!["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"]
                };
                let reg = regs.get(i).copied().unwrap_or("%rax");
                match arg {
                    Arg::N(n) => self.line(format!("movabsq ${n}, {reg}")),
                    Arg::Label(l) => self.line(format!("leaq {l}(%rip), {reg}")),
                }
                if i >= regs.len() {
                    self.line(format!("movq %rax, {}(%rsp)", 32 + (i - 4) * 8));
                }
            }
        }
        self.line(format!(
            "{} {}",
            if self.target == Target::MacArm64 {
                "bl"
            } else {
                "call"
            },
            self.symbol(name)
        ));
    }
    fn branch(&mut self, label: &str) {
        self.line(format!(
            "{} {label}",
            if self.target == Target::MacArm64 {
                "b"
            } else {
                "jmp"
            }
        ));
    }
    fn branch_false(&mut self, label: &str) {
        self.call("otp_truth", &[]);
        if self.target == Target::MacArm64 {
            self.line(format!("cbz x0, {label}"))
        } else {
            self.line(format!("testq %rax, %rax\njz {label}"))
        }
    }
    fn err<T>(&self, m: impl Into<String>) -> Result<T, Error> {
        Err(Error::new(self.line, 1, m))
    }
    fn resolve(&self, name: &str) -> Option<(usize, Sym)> {
        let mut s = self.current;
        let mut depth = 0;
        loop {
            if let Some(v) = self.scopes[s].syms.get(name) {
                return Some((depth, v.clone()));
            }
            s = self.scopes[s].parent?;
            depth += 1;
        }
    }
    fn routine(&self, name: &str) -> Option<(usize, usize, Vec<Param>, Type)> {
        let mut s = self.current;
        let mut d = 0;
        loop {
            if let Some(Sym::Routine(id, p, t)) = self.scopes[s].syms.get(name) {
                return Some((d, *id, p.clone(), t.clone()));
            }
            s = self.scopes[s].parent?;
            d += 1;
        }
    }
    fn insert(&mut self, s: usize, n: String, v: Sym) -> Result<(), Error> {
        if self.scopes[s].syms.contains_key(&n) {
            return self.err(format!("Duplicate identifier '{n}'"));
        }
        self.scopes[s].syms.insert(n, v);
        Ok(())
    }
    fn variable(&mut self, s: usize, n: String, t: Type) -> Result<usize, Error> {
        let slot = self.scopes[s].slots;
        self.insert(s, n.clone(), Sym::Var(slot, t.clone()))?;
        self.scopes[s].slots += 1;
        self.scopes[s].decls.push((slot, n, t));
        Ok(slot)
    }
    fn temp(&mut self, ty: Type) -> usize {
        let n = self.scopes[self.current].slots;
        self.scopes[self.current].slots += 1;
        self.scopes[self.current]
            .decls
            .push((n, format!("$temp{n}"), ty));
        n
    }
    fn addr(&mut self, e: &Expr) -> Result<Type, Error> {
        match e {
            Expr::Name(n) => {
                if let Some((d, Sym::Var(i, t))) = self.resolve(n) {
                    self.call("otp_ref", &[Arg::N(d as i64), Arg::N(i as i64)]);
                    Ok(t)
                } else {
                    self.err(format!("Variable expected: {n}"))
                }
            }
            Expr::Index(a, b) => {
                let t = self.addr(a)?;
                let idx = self.expr(b)?;
                if !idx.ordinal() {
                    return self.err("Ordinal array index expected");
                }
                match t {
                    Type::Array(_, _, t) => {
                        self.call("otp_index", &[]);
                        Ok(*t)
                    }
                    Type::Str(_) => self.err("String index assignment is not implemented"),
                    _ => self.err("Array expected"),
                }
            }
            Expr::Field(a, n) => {
                let t = self.addr(a)?;
                if let Type::Record(fs) = t {
                    if let Some((i, (_, ty))) = fs.iter().enumerate().find(|(_, (f, _))| f == n) {
                        self.call("otp_field", &[Arg::N(i as i64)]);
                        Ok(ty.clone())
                    } else {
                        self.err(format!("Unknown record field {n}"))
                    }
                } else {
                    self.err("Record expected")
                }
            }
            _ => self.err("Assignable variable expected"),
        }
    }
    fn validate_const(&mut self, e: &Expr, chain: &mut Vec<(usize, String)>) -> Result<(), Error> {
        match e {
            Expr::Name(n) => {
                let (depth, sym) = self
                    .resolve(n)
                    .ok_or_else(|| Error::new(self.line, 1, format!("Unknown constant '{n}'")))?;
                let mut owner = self.current;
                for _ in 0..depth {
                    owner = self.scopes[owner].parent.unwrap();
                }
                if let Sym::Const(x) = sym {
                    let key = (owner, n.clone());
                    if chain.contains(&key) {
                        return self.err("Circular constant");
                    }
                    chain.push(key);
                    let previous = self.current;
                    self.current = owner;
                    let result = self.validate_const(&x, chain);
                    self.current = previous;
                    chain.pop();
                    result
                } else {
                    self.err("Constant expression cannot reference a variable or routine")
                }
            }
            Expr::Unary(_, x) => self.validate_const(x, chain),
            Expr::Binary(_, a, b) => {
                self.validate_const(a, chain)?;
                self.validate_const(b, chain)
            }
            _ => Ok(()),
        }
    }
    fn expr(&mut self, e: &Expr) -> Result<Type, Error> {
        match e {
            Expr::Int(n) => {
                if *n < -2147483648 || *n > 2147483647 {
                    return self.err("Integer literal outside LongInt range");
                }
                self.call("otp_int", &[Arg::N(*n)]);
                Ok(if (-32768..=32767).contains(n) {
                    Type::integer()
                } else {
                    Type::Int(32, true)
                })
            }
            Expr::Real(n) => {
                self.call("otp_real", &[Arg::N(n.to_bits() as i64)]);
                Ok(Type::Real)
            }
            Expr::Bool(b) => {
                self.call("otp_bool", &[Arg::N(*b as i64)]);
                Ok(Type::Bool)
            }
            Expr::Str(s) => {
                if s.len() == 1 {
                    self.call("otp_char", &[Arg::N(s[0] as i64)]);
                    return Ok(Type::Char);
                }
                let a = self.literal(s);
                self.call("otp_string", &[a, Arg::N(s.len() as i64)]);
                Ok(Type::Str(s.len()))
            }
            Expr::Name(n) => match self.resolve(n) {
                Some((_, Sym::Var(..))) => {
                    let t = self.addr(e)?;
                    self.call("otp_load", &[]);
                    Ok(t)
                }
                Some((depth, Sym::Const(x))) => {
                    let previous = self.current;
                    let mut owner = previous;
                    for _ in 0..depth {
                        owner = self.scopes[owner].parent.unwrap();
                    }
                    let key = (owner, n.clone());
                    if self.constants.contains(&key) {
                        return self.err(format!("Circular constant '{n}'"));
                    }
                    self.constants.push(key);
                    self.current = owner;
                    self.type_cache.clear();
                    let result = self.expr(&x);
                    self.type_cache.clear();
                    self.current = previous;
                    self.constants.pop();
                    result
                }
                Some((_, Sym::Routine(..))) | None => self.invoke(n, &[], true),
            },
            Expr::Call(n, a) => self.invoke(n, a, true),
            Expr::Index(a, b) => {
                let ta = self.expr_type(a)?;
                if let Type::Str(_) = ta {
                    self.expr(a)?;
                    let t = self.expr(b)?;
                    if !t.ordinal() {
                        return self.err("Ordinal index expected");
                    }
                    self.call("otp_str_index", &[]);
                    Ok(Type::Char)
                } else {
                    let t = self.addr(e)?;
                    self.call("otp_load", &[]);
                    Ok(t)
                }
            }
            Expr::Field(..) => {
                let t = self.addr(e)?;
                self.call("otp_load", &[]);
                Ok(t)
            }
            Expr::Unary(op, x) => {
                let t = self.expr(x)?;
                if op == "not" {
                    if !matches!(t, Type::Int(..) | Type::Bool) {
                        return self.err("NOT requires Boolean or integer");
                    }
                    self.call("otp_not", &[]);
                } else {
                    if !t.numeric() {
                        return self.err("Unary sign requires numeric operand");
                    }
                    if op == "-" {
                        self.call("otp_neg", &[]);
                    }
                }
                Ok(t)
            }
            Expr::Binary(op, a, b) => {
                let ta = self.expr_type(a)?;
                let tb = self.expr_type(b)?;
                if (op == "and" || op == "or") && ta == Type::Bool && tb == Type::Bool {
                    let skip = self.label();
                    let end = self.label();
                    self.expr(a)?;
                    if op == "or" {
                        self.call("otp_not", &[]);
                    }
                    self.branch_false(&skip);
                    self.expr(b)?;
                    self.branch(&end);
                    self.line(format!("{skip}:"));
                    self.call("otp_bool", &[Arg::N((op == "or") as i64)]);
                    self.line(format!("{end}:"));
                    return Ok(Type::Bool);
                }
                let comparison = ["=", "<>", "<", ">", "<=", ">="].contains(&op.as_str());
                let arithmetic = ta.numeric() && tb.numeric();
                let text = ta.textlike() && tb.textlike();
                if !(arithmetic
                    || text && (op == "+" || comparison)
                    || ta == tb && ta == Type::Bool && comparison
                    || ta == tb && ta == Type::Bool && ["and", "or", "xor"].contains(&op.as_str()))
                {
                    return self.err(format!("Incompatible operands for {op}"));
                }
                if ["div", "mod", "and", "or", "xor", "shl", "shr"].contains(&op.as_str())
                    && (!matches!(ta, Type::Int(..)) || !matches!(tb, Type::Int(..)))
                    && ta != Type::Bool
                {
                    return self.err("Integer operands expected");
                }
                self.expr(a)?;
                self.expr(b)?;
                let id = [
                    "+", "-", "*", "/", "div", "mod", "=", "<>", "<", ">", "<=", ">=", "and", "or",
                    "xor", "shl", "shr",
                ]
                .iter()
                .position(|x| x == op)
                .unwrap();
                self.call("otp_binary", &[Arg::N(id as i64)]);
                Ok(if comparison {
                    Type::Bool
                } else if text {
                    Type::Str(255)
                } else if op == "/" || ta == Type::Real || tb == Type::Real {
                    Type::Real
                } else if ta == Type::Bool {
                    Type::Bool
                } else if ta == Type::Int(32, true) || tb == Type::Int(32, true) {
                    Type::Int(32, true)
                } else {
                    Type::integer()
                })
            }
        }
    }
    fn expr_type(&mut self, e: &Expr) -> Result<Type, Error> {
        let key = (self.current, e as *const Expr as usize);
        if let Some(t) = self.type_cache.get(&key) {
            return Ok(t.clone());
        }
        let saved = std::mem::take(&mut self.code);
        let data_len = self.data.len();
        let t = self.expr(e);
        self.code = saved;
        self.data.truncate(data_len);
        if let Ok(ty) = &t {
            self.type_cache.insert(key, ty.clone());
        }
        t
    }
    fn invoke(&mut self, n: &str, args: &[Expr], value: bool) -> Result<Type, Error> {
        if let Some((depth, id, ps, ret)) = self.routine(n) {
            if args.len() != ps.len() {
                return self.err(format!("{n} expects {} arguments", ps.len()));
            }
            for (a, p) in args.iter().zip(&ps) {
                let t = if p.by_ref {
                    self.addr(a)?
                } else {
                    self.expr(a)?
                };
                if !(if p.by_ref {
                    p.ty == t
                } else {
                    p.ty.accepts(&t)
                }) {
                    return self.err(format!("Argument type mismatch for {}", p.name));
                }
            }
            self.call("otp_parent", &[Arg::N(depth as i64)]);
            self.call(&format!("otp_user_{id}"), &[]);
            if value && ret == Type::Void {
                return self.err("Procedure has no value");
            }
            if !value && ret != Type::Void {
                self.call("otp_drop", &[]);
            }
            return Ok(ret);
        }
        let (min, max, refs, ret) = match n {
            "abs" | "sqr" => (1, 1, 0, Type::Int(32, true)),
            "sqrt" | "sin" | "cos" | "arctan" | "ln" | "exp" => (1, 1, 0, Type::Real),
            "round" | "trunc" => (1, 1, 0, Type::Int(32, true)),
            "length" | "ord" | "lo" | "hi" | "succ" | "pred" => (1, 1, 0, Type::integer()),
            "chr" | "upcase" => (1, 1, 0, Type::Char),
            "odd" => (1, 1, 0, Type::Bool),
            "copy" => (3, 3, 0, Type::Str(255)),
            "concat" => (1, 255, 0, Type::Str(255)),
            "pos" => (2, 2, 0, Type::integer()),
            "pi" => (0, 0, 0, Type::Real),
            "random" => (
                0,
                1,
                0,
                if args.is_empty() {
                    Type::Real
                } else {
                    Type::integer()
                },
            ),
            "randomize" => (0, 0, 0, Type::Void),
            "paramcount" | "ioresult" => (0, 0, 0, Type::integer()),
            "paramstr" => (1, 1, 0, Type::Str(255)),
            "inc" | "dec" => (1, 2, 1, Type::Void),
            "read" | "readln" => (if n == "read" { 1 } else { 0 }, 255, 2, Type::Void),
            "assign" => (2, 2, 1, Type::Void),
            "reset" | "rewrite" | "close" | "erase" | "flush" => (1, 1, 1, Type::Void),
            "eof" | "eoln" => (0, 1, 0, Type::Bool),
            "halt" => (0, 1, 0, Type::Void),
            "delay" | "textcolor" | "textbackground" => (1, 1, 0, Type::Void),
            "clrscr" | "clreol" => (0, 0, 0, Type::Void),
            "gotoxy" => (2, 2, 0, Type::Void),
            "wherex" | "wherey" => (0, 0, 0, Type::integer()),
            _ => return self.err(format!("Unknown or unimplemented routine '{n}'")),
        };
        if args.len() < min || args.len() > max {
            return self.err(format!("Wrong argument count for {n}: {}", args.len()));
        }
        let mut ts = vec![];
        for (i, a) in args.iter().enumerate() {
            ts.push(if refs == 2 || refs == 1 && i == 0 {
                self.addr(a)?
            } else {
                self.expr(a)?
            });
        }
        if [
            "abs", "sqr", "sqrt", "sin", "cos", "arctan", "ln", "exp", "round", "trunc",
        ]
        .contains(&n)
            && !ts[0].numeric()
        {
            return self.err(format!("{n} requires a numeric argument"));
        }
        if ["length", "copy", "pos", "concat"].contains(&n) && !ts[0].textlike() {
            return self.err(format!("{n} requires string data"));
        }
        if ["assign", "reset", "rewrite", "close", "erase", "flush"].contains(&n)
            && ts[0] != Type::Text
        {
            return self.err("Text file variable required");
        }
        if ["inc", "dec", "odd", "chr", "lo", "hi"].contains(&n) && !ts[0].ordinal() {
            return self.err("Ordinal argument required");
        }
        let integer = |i: usize| ts.get(i).map_or(false, |t| matches!(t, Type::Int(..)));
        let string = |i: usize| ts.get(i).map_or(false, Type::textlike);
        let valid = match n {
            "copy" => integer(1) && integer(2),
            "pos" => string(1),
            "concat" => ts.iter().all(Type::textlike),
            "assign" => string(1),
            "ord" => ts[0].ordinal(),
            "succ" | "pred" => matches!(ts[0], Type::Int(..) | Type::Char),
            "upcase" => ts[0] == Type::Char,
            "random" | "halt" => ts.is_empty() || integer(0),
            "paramstr" | "delay" | "textcolor" | "textbackground" | "chr" | "odd" | "lo" | "hi" => {
                integer(0)
            }
            "gotoxy" => integer(0) && integer(1),
            "inc" | "dec" => {
                matches!(ts[0], Type::Int(..) | Type::Char) && (ts.len() == 1 || integer(1))
            }
            "eof" | "eoln" => ts.is_empty() || ts[0] == Type::Text,
            "read" | "readln" => ts.iter().enumerate().all(|(i, t)| {
                i == 0 && *t == Type::Text
                    || matches!(t, Type::Int(..) | Type::Real | Type::Char | Type::Str(_))
            }),
            _ => true,
        };
        if !valid {
            return self.err(format!("Argument type mismatch for {n}"));
        }
        let ret = if ["abs", "sqr", "succ", "pred"].contains(&n) {
            ts[0].clone()
        } else {
            ret
        };
        let name = self.literal(n.as_bytes());
        self.call(
            "otp_builtin",
            &[name, Arg::N(n.len() as i64), Arg::N(args.len() as i64)],
        );
        if value && ret == Type::Void {
            return self.err("Procedure has no value");
        }
        if !value && ret != Type::Void {
            self.call("otp_drop", &[]);
        }
        Ok(ret)
    }
    fn statement(&mut self, s: &Stmt) -> Result<(), Error> {
        self.line = s.line;
        if self.opts.debug && !matches!(s.node, Node::Block(_) | Node::Empty) {
            self.call("otp_line", &[Arg::N(s.line as i64)]);
        }
        match &s.node {
            Node::Empty => (),
            Node::Block(v) => {
                for st in v {
                    self.statement(st)?
                }
            }
            Node::Assign(a, b) => {
                let ty = self.addr(a)?;
                let v = self.expr(b)?;
                if !ty.accepts(&v) {
                    return self.err(format!("Assignment type mismatch: {ty:?} <- {v:?}"));
                }
                self.call("otp_store", &[]);
            }
            Node::Call(n, a) => {
                self.invoke(n, a, false)?;
            }
            Node::Write(nl, args) => {
                let mut start = 0;
                if let Some((e, _, _)) = args.first() {
                    if self.expr_type(e)? == Type::Text {
                        self.expr(e)?;
                        self.call("otp_output_file", &[]);
                        start = 1;
                    }
                }
                for (e, w, d) in &args[start..] {
                    let t = self.expr(e)?;
                    if !matches!(
                        t,
                        Type::Int(..) | Type::Real | Type::Bool | Type::Char | Type::Str(_)
                    ) {
                        return self.err("Value cannot be written");
                    }
                    if let Some(w) = w {
                        if !matches!(self.expr(w)?, Type::Int(..)) {
                            return self.err("Integer field width required");
                        }
                    } else {
                        self.call("otp_int", &[Arg::N(0)]);
                    }
                    if let Some(d) = d {
                        if t != Type::Real {
                            return self.err("Decimal precision requires a Real value");
                        }
                        if !matches!(self.expr(d)?, Type::Int(..)) {
                            return self.err("Integer precision required");
                        }
                    } else {
                        self.call("otp_int", &[Arg::N(-1)]);
                    }
                    self.call("otp_write", &[]);
                }
                self.call("otp_write_end", &[Arg::N(*nl as i64)]);
            }
            Node::If(e, y, n) => {
                let no = self.label();
                let end = self.label();
                if self.expr(e)? != Type::Bool {
                    return self.err("Boolean condition required");
                }
                self.branch_false(&no);
                self.statement(y)?;
                self.branch(&end);
                self.line(format!("{no}:"));
                if let Some(n) = n {
                    self.statement(n)?;
                }
                self.line(format!("{end}:"));
            }
            Node::While(e, body) => {
                let top = self.label();
                let end = self.label();
                self.line(format!("{top}:"));
                if self.expr(e)? != Type::Bool {
                    return self.err("Boolean condition required");
                }
                self.branch_false(&end);
                self.loops.push((end.clone(), top.clone()));
                self.statement(body)?;
                self.loops.pop();
                self.branch(&top);
                self.line(format!("{end}:"));
            }
            Node::Repeat(v, e) => {
                let top = self.label();
                let test = self.label();
                let end = self.label();
                self.line(format!("{top}:"));
                self.loops.push((end.clone(), test.clone()));
                for st in v {
                    self.statement(st)?;
                }
                self.loops.pop();
                self.line(format!("{test}:"));
                if self.expr(e)? != Type::Bool {
                    return self.err("Boolean condition required");
                }
                self.branch_false(&top);
                self.line(format!("{end}:"));
            }
            Node::For(var, first, last, down, body) => {
                let ty = self.addr(var)?;
                if !matches!(ty, Type::Int(..)) {
                    return self.err("Integer FOR variable required");
                }
                if !self.expr(first)?.ordinal() {
                    return self.err("Ordinal FOR bound required");
                }
                self.call("otp_store", &[]);
                let slot = self.temp(ty);
                self.call("otp_ref", &[Arg::N(0), Arg::N(slot as i64)]);
                if !self.expr(last)?.ordinal() {
                    return self.err("Ordinal FOR bound required");
                }
                self.call("otp_store", &[]);
                let top = self.label();
                let step = self.label();
                let end = self.label();
                self.line(format!("{top}:"));
                self.expr(var)?;
                self.call("otp_ref", &[Arg::N(0), Arg::N(slot as i64)]);
                self.call("otp_load", &[]);
                self.call("otp_binary", &[Arg::N(if *down { 11 } else { 10 })]);
                self.branch_false(&end);
                self.loops.push((end.clone(), step.clone()));
                self.statement(body)?;
                self.loops.pop();
                self.line(format!("{step}:"));
                self.expr(var)?;
                self.call("otp_ref", &[Arg::N(0), Arg::N(slot as i64)]);
                self.call("otp_load", &[]);
                self.call("otp_binary", &[Arg::N(7)]);
                self.branch_false(&end);
                self.addr(var)?;
                self.expr(var)?;
                self.call("otp_int", &[Arg::N(1)]);
                self.call("otp_binary", &[Arg::N(if *down { 1 } else { 0 })]);
                self.call("otp_store", &[]);
                self.branch(&top);
                self.line(format!("{end}:"));
            }
            Node::Case(e, arms, otherwise) => {
                let slot = self.temp(Type::Int(32, true));
                self.call("otp_ref", &[Arg::N(0), Arg::N(slot as i64)]);
                if !self.expr(e)?.ordinal() {
                    return self.err("Ordinal CASE expression required");
                }
                self.call("otp_store", &[]);
                let end = self.label();
                for (ranges, body) in arms {
                    let selected = self.label();
                    let next = self.label();
                    for (lo, hi) in ranges {
                        let skip = self.label();
                        self.call("otp_case", &[Arg::N(slot as i64), Arg::N(*lo), Arg::N(*hi)]);
                        self.branch_false(&skip);
                        self.branch(&selected);
                        self.line(format!("{skip}:"));
                    }
                    self.branch(&next);
                    self.line(format!("{selected}:"));
                    self.statement(body)?;
                    self.branch(&end);
                    self.line(format!("{next}:"));
                }
                for st in otherwise {
                    self.statement(st)?;
                }
                self.line(format!("{end}:"));
            }
            Node::Break | Node::Continue => {
                if let Some((a, b)) = self.loops.last().cloned() {
                    self.branch(if matches!(s.node, Node::Break) {
                        &a
                    } else {
                        &b
                    });
                } else {
                    return self.err("BREAK/CONTINUE outside loop");
                }
            }
            Node::Exit => {
                self.branch(&self.ret.clone());
            }
        }
        Ok(())
    }
    fn register(
        &mut self,
        b: &Block,
        parent: Option<usize>,
        params: &[Param],
        func: Option<(&str, &Type)>,
    ) -> Result<usize, Error> {
        let id = self.scopes.len();
        self.scopes.push(Scope {
            parent,
            syms: HashMap::new(),
            slots: 0,
            decls: vec![],
        });
        for p in params {
            self.variable(id, p.name.clone(), p.ty.clone())?;
        }
        if let Some((n, t)) = func {
            self.variable(id, n.into(), t.clone())?;
        }
        for (n, e) in &b.constants {
            if !constant(e) {
                return self.err(format!("Constant expression required for {n}"));
            }
            self.insert(id, n.clone(), Sym::Const(e.clone()))?;
        }
        for (n, t) in &b.vars {
            self.variable(id, n.clone(), t.clone())?;
        }
        for r in &b.routines {
            let child = self.register(
                &r.block,
                Some(id),
                &r.params,
                if r.result != Type::Void {
                    Some((&r.name, &r.result))
                } else {
                    None
                },
            )?;
            self.insert(
                id,
                r.name.clone(),
                Sym::Routine(child, r.params.clone(), r.result.clone()),
            )?;
        }
        Ok(id)
    }
    fn body(&mut self, b: &Block, id: usize, nparams: usize, result: bool) -> Result<(), Error> {
        self.current = id;
        self.ret = self.label();
        self.loops.clear();
        let saved = std::mem::take(&mut self.code);
        self.statement(&b.body)?;
        let body = std::mem::take(&mut self.code);
        self.code = saved;
        let name = if id == 0 {
            "main".into()
        } else {
            format!("otp_user_{id}")
        };
        let name = self.symbol(&name);
        self.line(format!(".text\n.p2align 4\n.globl {name}\n{name}:"));
        if self.target == Target::MacArm64 {
            self.line("stp x29, x30, [sp, #-16]!\nmov x29, sp");
        } else {
            self.line("pushq %rbp\nmovq %rsp, %rbp\nsubq $48, %rsp");
        }
        if id == 0 {
            self.call(
                "otp_init",
                &[
                    Arg::N(self.opts.range as i64),
                    Arg::N(self.opts.overflow as i64),
                ],
            );
        }
        self.call(
            "otp_enter",
            &[Arg::N(self.scopes[id].slots as i64), Arg::N(nparams as i64)],
        );
        for (slot, name, ty) in self.scopes[id].decls.clone() {
            let desc = ty.encode();
            let d = self.literal(desc.as_bytes());
            let n = self.literal(name.as_bytes());
            self.call(
                "otp_decl",
                &[
                    Arg::N(slot as i64),
                    d,
                    Arg::N(desc.len() as i64),
                    n,
                    Arg::N(name.len() as i64),
                ],
            );
        }
        self.code += &body;
        self.line(format!("{}:", self.ret));
        self.call(
            "otp_leave",
            &[Arg::N(if result { nparams as i64 } else { -1 })],
        );
        if id == 0 {
            self.call("otp_finish", &[]);
        }
        if self.target == Target::MacArm64 {
            self.line("mov x0, #0\nldp x29, x30, [sp], #16\nret");
        } else {
            self.line("xorl %eax, %eax\nleave\nret");
        }
        for r in &b.routines {
            let child = if let Sym::Routine(c, _, _) = self.scopes[id].syms[&r.name] {
                c
            } else {
                unreachable!()
            };
            self.body(&r.block, child, r.params.len(), r.result != Type::Void)?;
        }
        Ok(())
    }
}
fn constant(e: &Expr) -> bool {
    match e {
        Expr::Int(_) | Expr::Real(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Name(_) => true,
        Expr::Unary(_, a) => constant(a),
        Expr::Binary(_, a, b) => constant(a) && constant(b),
        _ => false,
    }
}
pub fn generate(p: &Program, target: Target, opts: &Options) -> Result<String, Error> {
    for u in &p.uses {
        if !["system", "crt", "dos", "strings"].contains(&u.as_str()) {
            return Err(Error::new(
                1,
                1,
                format!("Unit '{u}' not implemented. No empty unit substitution."),
            ));
        }
    }
    let mut g = Gen {
        target,
        opts: opts.clone(),
        scopes: vec![],
        current: 0,
        code: String::new(),
        data: vec![],
        serial: 0,
        loops: vec![],
        ret: String::new(),
        line: 1,
        constants: vec![],
        type_cache: HashMap::new(),
    };
    g.register(&p.block, None, &[], None)?;
    for id in 0..g.scopes.len() {
        g.current = id;
        let constants: Vec<_> = g.scopes[id]
            .syms
            .iter()
            .filter_map(|(n, v)| {
                if matches!(v, Sym::Const(_)) {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect();
        for n in constants {
            g.validate_const(&Expr::Name(n), &mut vec![])?;
        }
    }
    g.body(&p.block, 0, 0, false)?;
    g.line(if target.apple() {
        ".section __TEXT,__const"
    } else {
        ".section .rodata"
    });
    for (n, bytes) in g.data.clone() {
        g.line(format!("{n}:"));
        let mut b = bytes;
        b.push(0);
        for chunk in b.chunks(24) {
            g.line(format!(
                ".byte {}",
                chunk
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
    }
    if target == Target::LinuxX64 {
        g.line(".section .note.GNU-stack,\"\",@progbits");
    }
    Ok(g.code)
}
