// SPDX-License-Identifier: GPL-3.0-only
//! Native Rust runtime for the implemented OTP Pascal language subset.
use otp_types::{Real48, Type};
use std::{
    cell::RefCell,
    collections::HashSet,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    net::TcpStream,
    rc::Rc,
};
type Cell = Rc<RefCell<Variable>>;
#[derive(Clone)]
enum V {
    Int(i64, u8, bool),
    Real(Real48),
    Bool(bool),
    Char(u8),
    Str(Vec<u8>),
    Array(Vec<Cell>),
    Record(Vec<Cell>),
    Text(Rc<RefCell<TextFile>>),
    Ref(Cell),
    Void,
}
struct Variable {
    ty: Type,
    name: String,
    value: V,
}
#[derive(Default)]
struct Scanner {
    line: String,
    pos: usize,
}
#[derive(Default)]
struct TextFile {
    path: String,
    reader: Option<BufReader<File>>,
    writer: Option<BufWriter<File>>,
    scan: Scanner,
}
struct Frame {
    parent: Option<usize>,
    vars: Vec<Cell>,
    base: usize,
}
struct Debugger {
    stream: TcpStream,
    mode: u8,
    depth: usize,
    breakpoints: HashSet<usize>,
}
struct Engine {
    stack: Vec<V>,
    frames: Vec<Frame>,
    parent: Option<usize>,
    range: bool,
    overflow: bool,
    output: Option<Rc<RefCell<TextFile>>>,
    stdin: Scanner,
    seed: u64,
    x: usize,
    y: usize,
    fg: i64,
    bg: i64,
    debug: Option<Debugger>,
    line: usize,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            stack: vec![],
            frames: vec![],
            parent: None,
            range: false,
            overflow: false,
            output: None,
            stdin: Scanner::default(),
            seed: 1,
            x: 1,
            y: 1,
            fg: 7,
            bg: 0,
            debug: None,
            line: 0,
        }
    }
}
thread_local! {static ENGINE:RefCell<Engine>=RefCell::new(Engine::default());}
fn engine<T>(f: impl FnOnce(&mut Engine) -> T) -> T {
    ENGINE.with(|e| f(&mut e.borrow_mut()))
}
fn fail(code: i32, msg: impl AsRef<str>) -> ! {
    eprintln!("Runtime error {code}: {}", msg.as_ref());
    std::process::exit(code)
}
fn io_check<T>(r: io::Result<T>) -> T {
    r.unwrap_or_else(|e| fail(101, e.to_string()))
}
fn cell(t: Type, n: String, v: V) -> Cell {
    Rc::new(RefCell::new(Variable {
        ty: t,
        name: n,
        value: v,
    }))
}
fn default(t: &Type) -> V {
    match t {
        Type::Int(b, s) => V::Int(0, *b, *s),
        Type::Real => V::Real(Real48([0; 6])),
        Type::Bool => V::Bool(false),
        Type::Char => V::Char(0),
        Type::Str(_) => V::Str(vec![]),
        Type::Text => V::Text(Rc::new(RefCell::new(TextFile::default()))),
        Type::Array(l, h, t) => V::Array(
            (*l..=*h)
                .map(|_| cell(*t.clone(), String::new(), default(t)))
                .collect(),
        ),
        Type::Record(fs) => V::Record(
            fs.iter()
                .map(|(n, t)| cell(t.clone(), n.clone(), default(t)))
                .collect(),
        ),
        Type::Void => V::Void,
    }
}
fn copy(v: &V) -> V {
    match v {
        V::Ref(c) => copy(&c.borrow().value),
        V::Array(a) => V::Array(
            a.iter()
                .map(|c| {
                    let c = c.borrow();
                    cell(c.ty.clone(), c.name.clone(), copy(&c.value))
                })
                .collect(),
        ),
        V::Record(a) => V::Record(
            a.iter()
                .map(|c| {
                    let c = c.borrow();
                    cell(c.ty.clone(), c.name.clone(), copy(&c.value))
                })
                .collect(),
        ),
        _ => v.clone(),
    }
}
fn int(v: &V) -> i64 {
    match v {
        V::Int(n, _, _) => *n,
        V::Bool(b) => *b as i64,
        V::Char(c) => *c as i64,
        V::Ref(c) => int(&c.borrow().value),
        _ => fail(201, "Ordinal value required"),
    }
}
fn real(v: &V) -> f64 {
    match v {
        V::Real(r) => r.to_f64(),
        V::Ref(c) => real(&c.borrow().value),
        _ => int(v) as f64,
    }
}
fn real_value(n: f64) -> V {
    V::Real(Real48::from_f64(n).unwrap_or_else(|e| fail(205, e)))
}
fn text(v: &V) -> Vec<u8> {
    match v {
        V::Str(s) => s.clone(),
        V::Char(c) => vec![*c],
        V::Int(n, ..) => n.to_string().into_bytes(),
        V::Real(r) => format!("{}", r.to_f64()).into_bytes(),
        V::Bool(b) => {
            if *b {
                b"TRUE".to_vec()
            } else {
                b"FALSE".to_vec()
            }
        }
        V::Ref(c) => text(&c.borrow().value),
        _ => fail(201, "Scalar value required"),
    }
}
fn normalize(n: i128, b: u8, s: bool, check: bool) -> V {
    let (min, max) = if s {
        (-(1i128 << (b - 1)), (1i128 << (b - 1)) - 1)
    } else {
        (0, (1i128 << b) - 1)
    };
    if check && (n < min || n > max) {
        fail(215, "Arithmetic overflow")
    }
    let mask = (1i128 << b) - 1;
    let mut k = n & mask;
    if s && k & (1i128 << (b - 1)) != 0 {
        k -= 1i128 << b;
    }
    V::Int(k as i64, b, s)
}
fn convert(v: V, t: &Type, range: bool) -> V {
    let v = copy(&v);
    match t {
        Type::Int(b, s) => {
            let n = int(&v) as i128;
            let (min, max) = if *s {
                (-(1i128 << (*b - 1)), (1i128 << (*b - 1)) - 1)
            } else {
                (0, (1i128 << *b) - 1)
            };
            if range && (n < min || n > max) {
                fail(201, "Range check error")
            }
            normalize(n, *b, *s, false)
        }
        Type::Real => real_value(real(&v)),
        Type::Bool => match v {
            V::Bool(_) => v,
            _ => fail(201, "Boolean assignment required"),
        },
        Type::Char => {
            let s = text(&v);
            if s.len() != 1 {
                fail(201, "Single character required")
            }
            V::Char(s[0])
        }
        Type::Str(n) => {
            let mut s = text(&v);
            s.truncate(*n);
            V::Str(s)
        }
        Type::Array(l, h, _) => {
            if let V::Array(a) = &v {
                if a.len() == (h - l + 1) as usize {
                    return v;
                }
            }
            fail(201, "Array assignment mismatch")
        }
        Type::Record(fs) => {
            if let V::Record(a) = &v {
                if a.len() == fs.len() {
                    return v;
                }
            }
            fail(201, "Record assignment mismatch")
        }
        Type::Text => {
            if matches!(v, V::Text(_)) {
                v
            } else {
                fail(201, "Text file required")
            }
        }
        Type::Void => v,
    }
}
impl Engine {
    fn pop(&mut self) -> V {
        self.stack
            .pop()
            .unwrap_or_else(|| fail(216, "Runtime expression stack underflow"))
    }
    fn pop_ref(&mut self) -> Cell {
        match self.pop() {
            V::Ref(c) => c,
            _ => fail(216, "Runtime reference required"),
        }
    }
    fn ancestor(&self, depth: usize) -> usize {
        let mut n = self
            .frames
            .len()
            .checked_sub(1)
            .unwrap_or_else(|| fail(216, "No active frame"));
        for _ in 0..depth {
            n = self.frames[n]
                .parent
                .unwrap_or_else(|| fail(216, "Invalid lexical frame"));
        }
        n
    }
    fn var(&self, depth: usize, slot: usize) -> Cell {
        self.frames[self.ancestor(depth)]
            .vars
            .get(slot)
            .cloned()
            .unwrap_or_else(|| fail(216, "Invalid variable slot"))
    }
    fn output(&mut self, s: &[u8]) {
        if let Some(f) = &self.output {
            let mut f = f.borrow_mut();
            let w = f
                .writer
                .as_mut()
                .unwrap_or_else(|| fail(105, "File not open for output"));
            io_check(w.write_all(s));
        } else {
            io_check(io::stdout().write_all(s));
            io_check(io::stdout().flush());
            for c in s {
                if *c == b'\n' {
                    self.x = 1;
                    self.y += 1
                } else if *c == b'\r' {
                    self.x = 1
                } else {
                    self.x += 1
                }
            }
        }
    }
    fn escape(&mut self, s: &[u8]) {
        let (x, y) = (self.x, self.y);
        self.output(s);
        self.x = x;
        self.y = y;
    }
    fn store(&self, c: Cell, v: V) {
        let ty = c.borrow().ty.clone();
        let v = convert(v, &ty, self.range);
        c.borrow_mut().value = v;
    }
}
unsafe fn bytes<'a>(p: *const u8, n: i64) -> &'a [u8] {
    if n < 0 || n > 16777216 || p.is_null() {
        fail(216, "Invalid runtime string descriptor")
    }
    std::slice::from_raw_parts(p, n as usize)
}
#[no_mangle]
pub extern "C" fn otp_init(range: i64, overflow: i64) {
    engine(|e| {
        *e = Engine::default();
        e.range = range != 0;
        e.overflow = overflow != 0;
        if let Ok(addr) = std::env::var("OTP_DEBUG_ADDR") {
            let addr: std::net::SocketAddr = addr
                .parse()
                .unwrap_or_else(|_| fail(216, "Invalid debugger endpoint"));
            if !addr.ip().is_loopback() {
                fail(216, "Debugger endpoint must be local")
            }
            let stream = TcpStream::connect(addr).unwrap_or_else(|e| fail(216, e.to_string()));
            e.debug = Some(Debugger {
                stream,
                mode: 1,
                depth: 0,
                breakpoints: HashSet::new(),
            });
        }
    })
}
#[no_mangle]
pub extern "C" fn otp_parent(depth: i64) {
    engine(|e| e.parent = Some(e.ancestor(depth as usize)))
}
#[no_mangle]
pub extern "C" fn otp_enter(slots: i64, params: i64) {
    engine(|e| {
        if e.frames.len() > 4096 {
            fail(202, "Stack overflow")
        }
        let base = e
            .stack
            .len()
            .checked_sub(params as usize)
            .unwrap_or_else(|| fail(216, "Missing parameters"));
        let args = e.stack.split_off(base);
        let mut vars = vec![];
        for v in args {
            vars.push(match v {
                V::Ref(c) => c,
                _ => cell(Type::Void, String::new(), v),
            });
        }
        while vars.len() < slots as usize {
            vars.push(cell(Type::Void, String::new(), V::Void));
        }
        e.frames.push(Frame {
            parent: e.parent.take(),
            vars,
            base,
        });
    })
}
#[no_mangle]
pub unsafe extern "C" fn otp_decl(
    slot: i64,
    desc: *const u8,
    len: i64,
    name: *const u8,
    nlen: i64,
) {
    let t = Type::decode(
        std::str::from_utf8(bytes(desc, len))
            .unwrap_or_else(|_| fail(216, "Invalid type descriptor")),
    )
    .unwrap_or_else(|s| fail(216, s));
    let name = String::from_utf8_lossy(bytes(name, nlen)).into_owned();
    engine(|e| {
        let c = e.var(0, slot as usize);
        let mut c = c.borrow_mut();
        if matches!(c.value, V::Void) {
            c.value = default(&t)
        } else if c.ty != t {
            // Value parameters start as Void-typed cells. A var parameter already
            // has its declared type: preserve its identity and nested aliases.
            c.value = convert(c.value.clone(), &t, e.range);
        }
        c.ty = t;
        if c.name.is_empty() {
            c.name = name;
        }
    })
}
#[no_mangle]
pub extern "C" fn otp_leave(result: i64) {
    engine(|e| {
        let f = e.frames.pop().unwrap_or_else(|| fail(216, "Missing frame"));
        let v = if result >= 0 {
            Some(copy(&f.vars[result as usize].borrow().value))
        } else {
            None
        };
        e.stack.truncate(f.base);
        if let Some(v) = v {
            e.stack.push(v);
        }
    })
}
#[no_mangle]
pub extern "C" fn otp_finish() {
    io_check(io::stdout().flush());
}
#[no_mangle]
pub extern "C" fn otp_ref(depth: i64, slot: i64) {
    engine(|e| {
        let c = e.var(depth as usize, slot as usize);
        e.stack.push(V::Ref(c));
    })
}
#[no_mangle]
pub extern "C" fn otp_load() {
    engine(|e| {
        let c = e.pop_ref();
        let v = copy(&c.borrow().value);
        e.stack.push(v);
    })
}
#[no_mangle]
pub extern "C" fn otp_store() {
    engine(|e| {
        let v = e.pop();
        let c = e.pop_ref();
        e.store(c, v);
    })
}
#[no_mangle]
pub extern "C" fn otp_drop() {
    engine(|e| {
        e.pop();
    });
}
#[no_mangle]
pub extern "C" fn otp_int(n: i64) {
    engine(|e| {
        e.stack.push(V::Int(
            n,
            if (-32768..=32767).contains(&n) {
                16
            } else {
                32
            },
            true,
        ))
    })
}
#[no_mangle]
pub extern "C" fn otp_real(bits: i64) {
    engine(|e| e.stack.push(real_value(f64::from_bits(bits as u64))))
}
#[no_mangle]
pub extern "C" fn otp_char(n: i64) {
    engine(|e| e.stack.push(V::Char(n as u8)))
}
#[no_mangle]
pub extern "C" fn otp_bool(n: i64) {
    engine(|e| e.stack.push(V::Bool(n != 0)))
}
#[no_mangle]
pub unsafe extern "C" fn otp_string(p: *const u8, n: i64) {
    let s = bytes(p, n).to_vec();
    engine(|e| e.stack.push(V::Str(s)))
}
#[no_mangle]
pub extern "C" fn otp_index() {
    engine(|e| {
        let idx = int(&e.pop());
        let c = e.pop_ref();
        let c = c.borrow();
        let lo = if let Type::Array(l, _, _) = c.ty {
            l
        } else {
            fail(216, "Array descriptor required")
        };
        let a = if let V::Array(a) = &c.value {
            a
        } else {
            fail(216, "Array reference required")
        };
        let n = idx
            .checked_sub(lo)
            .filter(|n| *n >= 0)
            .and_then(|n| a.get(n as usize))
            .cloned()
            .unwrap_or_else(|| fail(201, "Array index outside bounds"));
        e.stack.push(V::Ref(n));
    })
}
#[no_mangle]
pub extern "C" fn otp_field(index: i64) {
    engine(|e| {
        let c = e.pop_ref();
        let c = c.borrow();
        if let V::Record(fs) = &c.value {
            e.stack.push(V::Ref(
                fs.get(index as usize)
                    .cloned()
                    .unwrap_or_else(|| fail(216, "Invalid record field")),
            ))
        } else {
            fail(216, "Record required")
        }
    })
}
#[no_mangle]
pub extern "C" fn otp_str_index() {
    engine(|e| {
        let i = int(&e.pop());
        let s = text(&e.pop());
        let ch = if i == 0 {
            s.len() as u8
        } else {
            *s.get((i - 1) as usize)
                .unwrap_or_else(|| fail(201, "String index outside bounds"))
        };
        e.stack.push(V::Char(ch));
    })
}
#[no_mangle]
pub extern "C" fn otp_truth() -> i64 {
    engine(|e| match e.pop() {
        V::Bool(b) => b as i64,
        _ => fail(201, "Boolean condition required"),
    })
}
#[no_mangle]
pub extern "C" fn otp_not() {
    engine(|e| {
        let v = e.pop();
        e.stack.push(match v {
            V::Bool(b) => V::Bool(!b),
            V::Int(n, b, s) => normalize((!n) as i128, b, s, false),
            _ => fail(201, "Invalid NOT operand"),
        });
    })
}
#[no_mangle]
pub extern "C" fn otp_neg() {
    engine(|e| {
        let v = e.pop();
        e.stack.push(match v {
            V::Int(n, b, s) => normalize(-(n as i128), b, s, e.overflow),
            V::Real(r) => real_value(-r.to_f64()),
            _ => fail(201, "Invalid negation"),
        });
    })
}
#[no_mangle]
pub extern "C" fn otp_binary(op: i64) {
    engine(|e| {
        let b = e.pop();
        let a = e.pop();
        let v = if (6..=11).contains(&op) {
            let ord = if matches!(a, V::Str(_) | V::Char(_)) && matches!(b, V::Str(_) | V::Char(_))
            {
                text(&a).cmp(&text(&b))
            } else {
                real(&a)
                    .partial_cmp(&real(&b))
                    .unwrap_or_else(|| fail(207, "Invalid comparison"))
            };
            V::Bool(match op {
                6 => ord.is_eq(),
                7 => !ord.is_eq(),
                8 => ord.is_lt(),
                9 => ord.is_gt(),
                10 => !ord.is_gt(),
                _ => !ord.is_lt(),
            })
        } else if op == 0 && matches!(a, V::Str(_) | V::Char(_)) {
            let mut s = text(&a);
            s.extend(text(&b));
            s.truncate(255);
            V::Str(s)
        } else if matches!(a, V::Real(_)) || matches!(b, V::Real(_)) || op == 3 {
            let (x, y) = (real(&a), real(&b));
            real_value(match op {
                0 => x + y,
                1 => x - y,
                2 => x * y,
                3 => {
                    if y == 0.0 {
                        fail(200, "Division by zero")
                    }
                    x / y
                }
                _ => fail(201, "Invalid real operator"),
            })
        } else if let (V::Bool(x), V::Bool(y)) = (&a, &b) {
            V::Bool(match op {
                12 => *x && *y,
                13 => *x || *y,
                14 => *x ^ *y,
                _ => fail(201, "Invalid Boolean operator"),
            })
        } else {
            let (x, y) = (int(&a) as i128, int(&b) as i128);
            let bits = match (&a, &b) {
                (V::Int(_, a, _), V::Int(_, b, _)) => (*a).max(*b).max(16),
                _ => 16,
            };
            let z = match op {
                0 => x + y,
                1 => x - y,
                2 => x * y,
                4 => {
                    if y == 0 {
                        fail(200, "Division by zero")
                    }
                    x / y
                }
                5 => {
                    if y == 0 {
                        fail(200, "Division by zero")
                    }
                    x % y
                }
                12 => x & y,
                13 => x | y,
                14 => x ^ y,
                15 => x.wrapping_shl((y as u32) & 31),
                16 => ((x as u128) & ((1u128 << bits) - 1)).wrapping_shr((y as u32) & 31) as i128,
                _ => fail(216, "Invalid operator"),
            };
            normalize(z, bits, true, e.overflow)
        };
        e.stack.push(v);
    })
}
#[no_mangle]
pub extern "C" fn otp_case(slot: i64, lo: i64, hi: i64) {
    engine(|e| {
        let c = e.var(0, slot as usize);
        let n = int(&c.borrow().value);
        e.stack.push(V::Bool(n >= lo && n <= hi));
    })
}
fn file(v: &V) -> Rc<RefCell<TextFile>> {
    match v {
        V::Text(f) => f.clone(),
        V::Ref(c) => file(&c.borrow().value),
        _ => fail(102, "File variable required"),
    }
}
#[no_mangle]
pub extern "C" fn otp_output_file() {
    engine(|e| {
        let v = e.pop();
        e.output = Some(file(&v));
    })
}
#[no_mangle]
pub extern "C" fn otp_write() {
    engine(|e| {
        let dec = int(&e.pop());
        let width = int(&e.pop()).clamp(-65535, 65535);
        let v = e.pop();
        let mut s = if let V::Real(r) = &v {
            if dec >= 0 {
                format!("{:.*}", dec.min(255) as usize, r.to_f64()).into_bytes()
            } else {
                format!("{:E}", r.to_f64()).into_bytes()
            }
        } else {
            text(&v)
        };
        if width > 0 && width as usize > s.len() {
            let mut pad = vec![b' '; width as usize - s.len()];
            pad.extend(s);
            s = pad;
        }
        e.output(&s);
    })
}
#[no_mangle]
pub extern "C" fn otp_write_end(nl: i64) {
    engine(|e| {
        if nl != 0 {
            e.output(if cfg!(windows) { b"\r\n" } else { b"\n" });
        }
        if let Some(f) = &e.output {
            if let Some(w) = &mut f.borrow_mut().writer {
                io_check(w.flush());
            }
        }
        e.output = None;
    })
}
fn input_line(scan: &mut Scanner, reader: Option<&mut BufReader<File>>) {
    scan.line.clear();
    let n = if let Some(r) = reader {
        io_check(r.read_line(&mut scan.line))
    } else {
        io_check(io::stdin().read_line(&mut scan.line))
    };
    if n == 0 {
        fail(106, "End of input")
    }
    while scan.line.ends_with(['\n', '\r']) {
        scan.line.pop();
    }
    scan.pos = 0;
}
fn read_values(
    scan: &mut Scanner,
    mut reader: Option<&mut BufReader<File>>,
    args: &[V],
    nl: bool,
    range: bool,
) {
    if scan.pos >= scan.line.len() {
        input_line(scan, reader.as_deref_mut());
    }
    for a in args {
        let c = if let V::Ref(c) = a {
            c.clone()
        } else {
            fail(201, "READ requires variable")
        };
        let ty = c.borrow().ty.clone();
        let raw = if matches!(ty, Type::Str(_)) {
            let s = scan.line[scan.pos..].to_string();
            scan.pos = scan.line.len();
            s
        } else if ty == Type::Char {
            if scan.pos >= scan.line.len() {
                input_line(scan, reader.as_deref_mut());
            }
            let s = scan.line[scan.pos..].chars().next().unwrap_or(' ');
            scan.pos += s.len_utf8();
            s.to_string()
        } else {
            loop {
                while scan.pos < scan.line.len()
                    && scan.line.as_bytes()[scan.pos].is_ascii_whitespace()
                {
                    scan.pos += 1;
                }
                if scan.pos < scan.line.len() {
                    break;
                }
                input_line(scan, reader.as_deref_mut());
            }
            let st = scan.pos;
            while scan.pos < scan.line.len()
                && !scan.line.as_bytes()[scan.pos].is_ascii_whitespace()
            {
                scan.pos += 1;
            }
            scan.line[st..scan.pos].to_string()
        };
        let v = match ty {
            Type::Int(..) => V::Int(
                raw.parse()
                    .unwrap_or_else(|_| fail(106, "Invalid integer input")),
                32,
                true,
            ),
            Type::Real => real_value(
                raw.parse()
                    .unwrap_or_else(|_| fail(106, "Invalid real input")),
            ),
            Type::Str(_) => V::Str(raw.into_bytes()),
            Type::Char => V::Char(*raw.as_bytes().first().unwrap_or(&b' ')),
            _ => fail(106, "Unsupported READ type"),
        };
        c.borrow_mut().value = convert(v, &ty, range);
    }
    if nl {
        scan.line.clear();
        scan.pos = 0;
    }
}
#[no_mangle]
pub unsafe extern "C" fn otp_builtin(p: *const u8, len: i64, argc: i64) {
    let name = String::from_utf8_lossy(bytes(p, len)).into_owned();
    engine(|e| {
        let base = e
            .stack
            .len()
            .checked_sub(argc as usize)
            .unwrap_or_else(|| fail(216, "Missing builtin args"));
        let args = e.stack.split_off(base);
        let a = |i: usize| args.get(i).unwrap_or_else(|| fail(216, "Missing argument"));
        let result = match name.as_str() {
            "abs" => Some(match copy(a(0)) {
                V::Real(r) => real_value(r.to_f64().abs()),
                V::Int(n, b, s) => normalize((n as i128).abs(), b, s, e.overflow),
                _ => fail(201, "ABS numeric argument"),
            }),
            "sqr" => Some(match copy(a(0)) {
                V::Real(r) => real_value(r.to_f64().powi(2)),
                V::Int(n, b, s) => normalize(n as i128 * n as i128, b, s, e.overflow),
                _ => fail(201, "SQR numeric argument"),
            }),
            "sqrt" | "sin" | "cos" | "arctan" | "ln" | "exp" => {
                let x = real(a(0));
                Some(real_value(match name.as_str() {
                    "sqrt" => x.sqrt(),
                    "sin" => x.sin(),
                    "cos" => x.cos(),
                    "arctan" => x.atan(),
                    "ln" => x.ln(),
                    _ => x.exp(),
                }))
            }
            "round" | "trunc" => {
                let x = real(a(0));
                let x = if name == "round" {
                    x.round()
                } else {
                    x.trunc()
                };
                if x < i32::MIN as f64 || x > i32::MAX as f64 {
                    fail(207, "Integer conversion overflow")
                }
                Some(V::Int(x as i64, 32, true))
            }
            "length" => Some(V::Int(text(a(0)).len() as i64, 16, true)),
            "ord" => Some(V::Int(int(a(0)), 16, true)),
            "chr" => Some(V::Char(int(a(0)) as u8)),
            "upcase" => {
                let t = text(a(0));
                if t.len() != 1 {
                    fail(201, "UPCASE expects character")
                }
                Some(V::Char(t[0].to_ascii_uppercase()))
            }
            "odd" => Some(V::Bool(int(a(0)) & 1 != 0)),
            "lo" => Some(V::Int(int(a(0)) & 255, 16, true)),
            "hi" => Some(V::Int((int(a(0)) >> 8) & 255, 16, true)),
            "succ" | "pred" => {
                let delta = if name == "succ" { 1 } else { -1 };
                Some(match copy(a(0)) {
                    V::Char(n) => V::Char((n as i64 + delta) as u8),
                    V::Int(n, b, s) => normalize((n + delta) as i128, b, s, e.range),
                    _ => fail(201, "Ordinal argument required"),
                })
            }
            "copy" => {
                let t = text(a(0));
                let start = int(a(1));
                let count = int(a(2)).max(0) as usize;
                Some(V::Str(if start < 1 || start as usize > t.len() {
                    vec![]
                } else {
                    t[(start - 1) as usize..]
                        .iter()
                        .take(count)
                        .copied()
                        .collect()
                }))
            }
            "concat" => {
                let mut s = vec![];
                for a in &args {
                    s.extend(text(a));
                }
                s.truncate(255);
                Some(V::Str(s))
            }
            "pos" => {
                let needle = text(a(0));
                let hay = text(a(1));
                Some(V::Int(
                    if needle.is_empty() {
                        0
                    } else {
                        hay.windows(needle.len())
                            .position(|w| w == needle)
                            .map(|x| x + 1)
                            .unwrap_or(0)
                    } as i64,
                    16,
                    true,
                ))
            }
            "pi" => Some(real_value(std::f64::consts::PI)),
            "randomize" => {
                e.seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;
                None
            }
            "random" => {
                e.seed = e.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let n = e.seed >> 33;
                Some(if args.is_empty() {
                    real_value(n as f64 / 2147483648.0)
                } else {
                    let bound = int(a(0));
                    if bound <= 0 {
                        fail(201, "RANDOM range must be positive")
                    }
                    V::Int((n % bound as u64) as i64, 16, true)
                })
            }
            "inc" | "dec" => {
                let c = if let V::Ref(c) = a(0) {
                    c.clone()
                } else {
                    fail(201, "Variable required")
                };
                let delta = if args.len() == 2 { int(a(1)) } else { 1 };
                let old = copy(&c.borrow().value);
                let n = int(&old) + if name == "inc" { delta } else { -delta };
                let v = match old {
                    V::Char(_) => V::Char(n as u8),
                    V::Int(_, b, s) => normalize(n as i128, b, s, e.overflow),
                    _ => fail(201, "Ordinal variable required"),
                };
                e.store(c, v);
                None
            }
            "paramcount" => Some(V::Int(
                std::env::args_os().count().saturating_sub(1) as i64,
                16,
                true,
            )),
            "paramstr" => Some(V::Str(
                std::env::args()
                    .nth(int(a(0)) as usize)
                    .unwrap_or_default()
                    .into_bytes(),
            )),
            "ioresult" => Some(V::Int(0, 16, true)),
            "assign" => {
                let f = file(a(0));
                let mut f = f.borrow_mut();
                if f.reader.is_some() || f.writer.is_some() {
                    fail(102, "Close file before Assign")
                }
                f.path = String::from_utf8_lossy(&text(a(1))).into_owned();
                None
            }
            "reset" | "rewrite" => {
                let f = file(a(0));
                let mut f = f.borrow_mut();
                f.reader = None;
                f.writer = None;
                f.scan = Scanner::default();
                if name == "reset" {
                    f.reader = Some(BufReader::new(io_check(File::open(&f.path))))
                } else {
                    f.writer = Some(BufWriter::new(io_check(File::create(&f.path))))
                }
                None
            }
            "close" | "flush" => {
                let f = file(a(0));
                let mut f = f.borrow_mut();
                if let Some(w) = &mut f.writer {
                    io_check(w.flush());
                }
                if name == "close" {
                    if f.reader.is_none() && f.writer.is_none() {
                        fail(103, "File is not open")
                    }
                    f.reader = None;
                    f.writer = None;
                }
                None
            }
            "erase" => {
                let f = file(a(0));
                let f = f.borrow();
                if f.reader.is_some() || f.writer.is_some() {
                    fail(102, "File must be closed before Erase")
                }
                io_check(std::fs::remove_file(&f.path));
                None
            }
            "read" | "readln" => {
                let mut start = 0;
                let f = args.first().and_then(|v| {
                    let v = copy(v);
                    if let V::Text(f) = v {
                        Some(f)
                    } else {
                        None
                    }
                });
                if let Some(f) = f {
                    start = 1;
                    let mut f = f.borrow_mut();
                    let TextFile { scan, reader, .. } = &mut *f;
                    let r = reader
                        .as_mut()
                        .unwrap_or_else(|| fail(104, "File not open for input"));
                    read_values(scan, Some(r), &args[start..], name == "readln", e.range);
                } else {
                    read_values(
                        &mut e.stdin,
                        None,
                        &args[start..],
                        name == "readln",
                        e.range,
                    );
                }
                None
            }
            "eof" | "eoln" => {
                if args.is_empty() {
                    Some(V::Bool(if name == "eoln" {
                        e.stdin.pos >= e.stdin.line.len()
                    } else {
                        fail(106, "EOF on standard input is not implemented")
                    }))
                } else {
                    let f = file(a(0));
                    let mut f = f.borrow_mut();
                    let buffered = f.scan.pos < f.scan.line.len();
                    if name == "eoln" {
                        Some(V::Bool(!buffered))
                    } else if buffered {
                        Some(V::Bool(false))
                    } else {
                        let r = f
                            .reader
                            .as_mut()
                            .unwrap_or_else(|| fail(104, "File not open for input"));
                        Some(V::Bool(io_check(r.fill_buf()).is_empty()))
                    }
                }
            }
            "halt" => std::process::exit(if args.is_empty() { 0 } else { int(a(0)) as i32 }),
            "delay" => {
                let n = int(a(0));
                if n < 0 {
                    fail(201, "Negative delay")
                }
                std::thread::sleep(std::time::Duration::from_millis(n as u64));
                None
            }
            "clrscr" => {
                e.output(b"\x1b[2J\x1b[H");
                e.x = 1;
                e.y = 1;
                None
            }
            "clreol" => {
                e.escape(b"\x1b[K");
                None
            }
            "gotoxy" => {
                let (x, y) = (int(a(0)), int(a(1)));
                if x < 1 || y < 1 {
                    fail(201, "Cursor coordinate outside range")
                }
                e.output(format!("\x1b[{y};{x}H").as_bytes());
                e.x = x as usize;
                e.y = y as usize;
                None
            }
            "wherex" => Some(V::Int(e.x as i64, 16, true)),
            "wherey" => Some(V::Int(e.y as i64, 16, true)),
            "textcolor" | "textbackground" => {
                let v = int(a(0));
                let ansi = [0, 4, 2, 6, 1, 5, 3, 7];
                let base = if name == "textcolor" {
                    e.fg = v;
                    if v & 8 != 0 {
                        90
                    } else {
                        30
                    }
                } else {
                    e.bg = v;
                    40
                };
                e.escape(format!("\x1b[{}m", base + ansi[(v & 7) as usize]).as_bytes());
                None
            }
            _ => fail(216, format!("Missing runtime routine {name}")),
        };
        if let Some(v) = result {
            e.stack.push(v);
        }
    })
}
#[no_mangle]
pub extern "C" fn otp_line(line: i64) {
    engine(|e| {
        e.line = line as usize;
        let depth = e.frames.len();
        let stop = e
            .debug
            .as_ref()
            .map(|d| {
                d.mode == 1
                    || d.mode == 2 && depth <= d.depth
                    || d.breakpoints.contains(&(line as usize))
            })
            .unwrap_or(false);
        if !stop {
            return;
        }
        let mut state = String::new();
        for frame in &e.frames {
            for c in &frame.vars {
                let c = c.borrow();
                if !c.name.starts_with('$') {
                    let value = match c.value {
                        V::Array(_) => "[array]".into(),
                        V::Record(_) => "[record]".into(),
                        V::Text(_) => "[text file]".into(),
                        _ => String::from_utf8_lossy(&text(&c.value))
                            .replace(['\n', '\r', '\t'], " "),
                    };
                    state += &format!("{}={value}; ", c.name);
                }
            }
        }
        let d = e.debug.as_mut().unwrap();
        io_check(
            d.stream
                .write_all(format!("LINE {line} {depth} {state}\n").as_bytes()),
        );
        loop {
            let mut cmd = String::new();
            loop {
                let mut b = [0];
                if d.stream.read(&mut b).unwrap_or(0) == 0 {
                    std::process::exit(0)
                }
                if b[0] == b'\n' {
                    break;
                }
                cmd.push(b[0] as char);
                if cmd.len() > 4096 {
                    fail(216, "Debugger command too long")
                }
            }
            let parts: Vec<_> = cmd.split_whitespace().collect();
            match parts.first().copied() {
                Some("STEP") => {
                    d.mode = 1;
                    break;
                }
                Some("NEXT") => {
                    d.mode = 2;
                    d.depth = depth;
                    break;
                }
                Some("CONT") => {
                    d.mode = 0;
                    break;
                }
                Some("BREAK") => {
                    if let Some(n) = parts.get(1).and_then(|n| n.parse().ok()) {
                        if !d.breakpoints.insert(n) {
                            d.breakpoints.remove(&n);
                        }
                    }
                }
                Some("QUIT") => std::process::exit(0),
                _ => {
                    io_check(
                        d.stream
                            .write_all(b"ERROR Supported commands: STEP NEXT CONT BREAK n QUIT\n"),
                    );
                }
            }
        }
    })
}
