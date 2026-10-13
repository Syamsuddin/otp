// SPDX-License-Identifier: GPL-3.0-only
//! OTP's native bitmap desktop editor. No browser or embedded web runtime.
mod desktop;
use desktop::{Browser, Drag, Rect};
use font8x8::{UnicodeFonts, BASIC_FONTS};
use minifb::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};
const COLS: usize = 100;
const ROWS: usize = 35;
const CW: usize = 10;
const CH: usize = 18;
const W: usize = COLS * CW;
const H: usize = ROWS * CH;
const BLUE: u32 = 0x0000aa;
const WHITE: u32 = 0xffffff;
const GRAY: u32 = 0xaaaaaa;
const BLACK: u32 = 0x000000;
const CYAN: u32 = 0x00aaaa;
const YELLOW: u32 = 0xffff55;
const RED: u32 = 0xaa0000;
const SAMPLE:&str="program HelloOdin;\nuses Crt;\n\nvar\n  Nama: string[40];\n\nbegin\n  WriteLn('Selamat datang di ODIN Turbo Pascal!');\n  Write('Nama Anda: ');\n  ReadLn(Nama);\n  WriteLn('Halo, ', Nama, '!');\nend.\n";
#[derive(Clone)]
struct Snapshot {
    lines: Vec<String>,
    row: usize,
    col: usize,
}
struct Document {
    rect: Rect,
    restore_rect: Option<Rect>,
    follow_cursor: bool,
    path: Option<PathBuf>,
    lines: Vec<String>,
    row: usize,
    col: usize,
    top: usize,
    left: usize,
    dirty: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    anchor: Option<(usize, usize)>,
}
impl Document {
    fn new(text: &str, path: Option<PathBuf>) -> Self {
        Self {
            rect: Rect::desktop(),
            restore_rect: None,
            follow_cursor: true,
            path,
            lines: text
                .replace("\r\n", "\n")
                .replace('\t', "  ")
                .split('\n')
                .map(str::to_string)
                .collect(),
            row: 0,
            col: 0,
            top: 0,
            left: 0,
            dirty: false,
            undo: vec![],
            redo: vec![],
            anchor: None,
        }
    }
    fn text(&self) -> String {
        self.lines.join("\n")
    }
    fn name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or("NONAME.PAS".into())
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            row: self.row,
            col: self.col,
        }
    }
    fn checkpoint(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }
    fn restore(&mut self, s: Snapshot) {
        self.lines = s.lines;
        self.row = s.row;
        self.col = s.col;
        self.anchor = None;
        self.dirty = true;
    }
    fn undo(&mut self) {
        if let Some(s) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(s);
        }
    }
    fn redo(&mut self) {
        if let Some(s) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(s);
        }
    }
    fn byte(&self, row: usize, col: usize) -> usize {
        self.lines[row]
            .char_indices()
            .nth(col)
            .map(|(i, _)| i)
            .unwrap_or(self.lines[row].len())
    }
    fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let p = (self.row, self.col);
        self.anchor
            .filter(|a| *a != p)
            .map(|a| if a < p { (a, p) } else { (p, a) })
    }
    fn selected(&self) -> String {
        if let Some(((r, c), (r2, c2))) = self.selection() {
            if r == r2 {
                return self.lines[r][self.byte(r, c)..self.byte(r, c2)].into();
            }
            let mut s = self.lines[r][self.byte(r, c)..].to_string();
            for n in r + 1..r2 {
                s.push('\n');
                s += &self.lines[n];
            }
            s.push('\n');
            s += &self.lines[r2][..self.byte(r2, c2)];
            s
        } else {
            String::new()
        }
    }
    fn delete_selection(&mut self) -> bool {
        if let Some(((r, c), (r2, c2))) = self.selection() {
            let a = self.lines[r][..self.byte(r, c)].to_string();
            let b = self.lines[r2][self.byte(r2, c2)..].to_string();
            self.lines.splice(r..=r2, [a + &b]);
            self.row = r;
            self.col = c;
            self.anchor = None;
            true
        } else {
            false
        }
    }
    fn insert(&mut self, text: &str) {
        self.checkpoint();
        self.delete_selection();
        let i = self.byte(self.row, self.col);
        let tail = self.lines[self.row].split_off(i);
        let parts: Vec<_> = text.split('\n').collect();
        self.lines[self.row] += parts[0];
        if parts.len() == 1 {
            self.col += parts[0].chars().count();
            self.lines[self.row] += &tail;
        } else {
            let at = self.row + 1;
            self.lines
                .splice(at..at, parts[1..].iter().map(|s| s.to_string()));
            self.row += parts.len() - 1;
            self.col = self.lines[self.row].chars().count();
            self.lines[self.row] += &tail;
        }
    }
    fn backspace(&mut self, delete: bool) {
        if self.selection().is_some() {
            self.checkpoint();
            self.delete_selection();
            return;
        }
        if delete {
            let len = self.lines[self.row].chars().count();
            if self.col < len {
                self.checkpoint();
                let i = self.byte(self.row, self.col);
                self.lines[self.row].remove(i);
            } else if self.row + 1 < self.lines.len() {
                self.checkpoint();
                let s = self.lines.remove(self.row + 1);
                self.lines[self.row] += &s;
            }
        } else if self.col > 0 {
            self.checkpoint();
            self.col -= 1;
            let i = self.byte(self.row, self.col);
            self.lines[self.row].remove(i);
        } else if self.row > 0 {
            self.checkpoint();
            let s = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.lines[self.row].chars().count();
            self.lines[self.row] += &s;
        }
    }
    fn movement(&mut self, key: Key, shift: bool, height: usize) {
        if shift {
            self.anchor.get_or_insert((self.row, self.col));
        } else {
            self.anchor = None;
        }
        match key {
            Key::Left => {
                if self.col > 0 {
                    self.col -= 1
                } else if self.row > 0 {
                    self.row -= 1;
                    self.col = self.lines[self.row].chars().count()
                }
            }
            Key::Right => {
                if self.col < self.lines[self.row].chars().count() {
                    self.col += 1
                } else if self.row + 1 < self.lines.len() {
                    self.row += 1;
                    self.col = 0
                }
            }
            Key::Up => self.row = self.row.saturating_sub(1),
            Key::Down => self.row = (self.row + 1).min(self.lines.len() - 1),
            Key::Home => self.col = 0,
            Key::End => self.col = self.lines[self.row].chars().count(),
            Key::PageUp => self.row = self.row.saturating_sub(height),
            Key::PageDown => self.row = (self.row + height).min(self.lines.len() - 1),
            _ => (),
        };
        self.col = self.col.min(self.lines[self.row].chars().count());
    }
    fn find(&mut self, needle: &str) -> bool {
        if needle.is_empty() {
            return false;
        }
        let total = self.lines.len();
        for offset in 0..=total {
            let r = (self.row + offset) % total;
            let start = if offset == 0 {
                self.byte(r, (self.col + 1).min(self.lines[r].chars().count()))
            } else {
                0
            };
            if let Some(i) = self.lines[r][start..]
                .to_ascii_lowercase()
                .find(&needle.to_ascii_lowercase())
            {
                self.row = r;
                self.col = self.lines[r][..start + i].chars().count();
                self.anchor = Some((r, self.col + needle.chars().count()));
                return true;
            }
        }
        false
    }
}
struct Canvas {
    pixels: Vec<u32>,
    logo: Option<(usize, usize, Vec<u8>)>,
}
impl Canvas {
    fn new() -> Self {
        let decoder = png::Decoder::new(std::io::Cursor::new(include_bytes!(
            "../../../assets/branding/odin-turbo-pascal-master-v1.png"
        )));
        let logo = decoder.read_info().ok().and_then(|mut reader| {
            let mut buf = vec![0; reader.output_buffer_size()];
            reader.next_frame(&mut buf).ok().and_then(|info| {
                let bytes = &buf[..info.buffer_size()];
                let rgb = match info.color_type {
                    png::ColorType::Rgb => bytes.to_vec(),
                    png::ColorType::Rgba => bytes
                        .chunks_exact(4)
                        .flat_map(|p| p[..3].to_vec())
                        .collect(),
                    _ => return None,
                };
                Some((info.width as usize, info.height as usize, rgb))
            })
        });
        Self {
            pixels: vec![BLUE; W * H],
            logo,
        }
    }
    fn logo(&mut self, x: usize, y: usize, size: usize) {
        if let Some((w, h, data)) = &self.logo {
            for yy in 0..size {
                for xx in 0..size {
                    let src = ((yy * h / size) * w + xx * w / size) * 3;
                    let dest = (y + yy) * W + x + xx;
                    if x + xx < W && y + yy < H {
                        self.pixels[dest] = ((data[src] as u32) << 16)
                            | ((data[src + 1] as u32) << 8)
                            | data[src + 2] as u32;
                    }
                }
            }
        }
    }
    fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for yy in y.min(H)..(y + h).min(H) {
            for xx in x.min(W)..(x + w).min(W) {
                self.pixels[yy * W + xx] = color;
            }
        }
    }
    fn cell(&mut self, x: usize, y: usize, c: char, fg: u32, bg: u32) {
        self.rect(x * CW, y * CH, CW, CH, bg);
        if let Some(g) = BASIC_FONTS.get(c).or_else(|| BASIC_FONTS.get('?')) {
            for (yy, bits) in g.iter().enumerate() {
                for xx in 0..8 {
                    if bits & (1 << xx) != 0 {
                        self.rect(x * CW + 1 + xx, y * CH + 1 + yy * 2, 1, 2, fg);
                    }
                }
            }
        }
    }
    fn text(&mut self, x: usize, y: usize, s: &str, fg: u32, bg: u32) {
        for (i, c) in s.chars().take(COLS.saturating_sub(x)).enumerate() {
            self.cell(x + i, y, c, fg, bg);
        }
    }
    fn bar(&mut self, y: usize, s: &str, fg: u32, bg: u32) {
        self.rect(0, y * CH, W, CH, bg);
        self.text(1, y, s, fg, bg);
    }
    fn box_at(&mut self, x: usize, y: usize, w: usize, h: usize, title: &str, fg: u32, bg: u32) {
        self.rect(x * CW, y * CH, w * CW, h * CH, bg);
        let (px, py) = (x * CW + 3, y * CH + 8);
        let (ww, hh) = (w * CW - 6, h * CH - 16);
        self.rect(px, py, ww, 1, fg);
        self.rect(px, py + hh, ww, 1, fg);
        self.rect(px, py, 1, hh, fg);
        self.rect(px + ww, py, 1, hh, fg);
        self.text(x + 2, y, &format!(" {title} "), fg, bg);
    }
    fn ppm(&self, p: &Path) -> std::io::Result<()> {
        let mut f = std::fs::File::create(p)?;
        write!(f, "P6\n{W} {H}\n255\n")?;
        for p in &self.pixels {
            f.write_all(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8])?;
        }
        Ok(())
    }
}
struct Chars(Sender<char>);
impl InputCallback for Chars {
    fn add_char(&mut self, c: u32) {
        if let Some(c) = char::from_u32(c) {
            if !c.is_control() {
                let _ = self.0.send(c);
            }
        }
    }
}
#[derive(Clone, Copy)]
enum Task {
    Build,
    Run,
    Debug,
}
enum Event {
    Output(String),
    Built(PathBuf),
    Error(usize, String),
    Paused(usize, String),
    Done(i32),
}
enum Control {
    Input(String),
    Debug(String),
    Stop,
}
struct Job {
    tx: Sender<Control>,
    handle: std::thread::JoinHandle<()>,
}
fn worker(
    source: PathBuf,
    task: Task,
    opts: otp_core::Options,
    tx: Sender<Event>,
    rx: Receiver<Control>,
) {
    let output = source.with_extension(if cfg!(windows) { "exe" } else { "" });
    let mut opts = opts;
    opts.debug = matches!(task, Task::Debug);
    let result = match otp_core::build::build(&source, &output, &opts) {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(Event::Error(e.line, e.to_string()));
            return;
        }
    };
    let _ = tx.send(Event::Built(result.executable.clone()));
    if matches!(task, Task::Build) {
        let _ = tx.send(Event::Done(0));
        return;
    }
    let listener = if matches!(task, Task::Debug) {
        match TcpListener::bind("127.0.0.1:0") {
            Ok(l) => {
                let _ = l.set_nonblocking(true);
                Some(l)
            }
            Err(e) => {
                let _ = tx.send(Event::Error(0, e.to_string()));
                return;
            }
        }
    } else {
        None
    };
    let mut command = Command::new(&result.executable);
    command
        .current_dir(source.parent().unwrap_or(Path::new(".")))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(l) = &listener {
        command.env("OTP_DEBUG_ADDR", l.local_addr().unwrap().to_string());
    }
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(Event::Error(0, e.to_string()));
            return;
        }
    };
    let mut input = child.stdin.take();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let pump = |mut pipe: Box<dyn Read + Send>, tx: Sender<Event>| {
        std::thread::spawn(move || {
            let mut b = [0; 1024];
            while let Ok(n) = pipe.read(&mut b) {
                if n == 0 {
                    break;
                }
                let _ = tx.send(Event::Output(String::from_utf8_lossy(&b[..n]).into_owned()));
            }
        })
    };
    let a = pump(Box::new(stdout), tx.clone());
    let b = pump(Box::new(stderr), tx.clone());
    let mut debug = None;
    let mut debug_reader = None;
    let start = Instant::now();
    loop {
        if debug.is_none() {
            if let Some(l) = &listener {
                if let Ok((s, _)) = l.accept() {
                    let r = s.try_clone().unwrap();
                    let tx = tx.clone();
                    debug_reader = Some(std::thread::spawn(move || {
                        for line in BufReader::new(r).lines().map_while(Result::ok) {
                            let mut parts = line.splitn(4, ' ');
                            if parts.next() == Some("LINE") {
                                let n = parts.next().and_then(|n| n.parse().ok()).unwrap_or(1);
                                let _depth = parts.next();
                                let _ =
                                    tx.send(Event::Paused(n, parts.next().unwrap_or("").into()));
                            }
                        }
                    }));
                    debug = Some(s);
                } else if start.elapsed().as_secs() > 15 {
                    let _ = child.kill();
                    let _ = tx.send(Event::Output("\nDebugger connection timeout\n".into()));
                }
            }
        }
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                Control::Input(s) => {
                    if let Some(i) = &mut input {
                        let _ = writeln!(i, "{s}");
                        let _ = i.flush();
                    }
                }
                Control::Debug(s) => {
                    if let Some(d) = &mut debug {
                        let _ = writeln!(d, "{s}");
                    }
                }
                Control::Stop => {
                    let _ = child.kill();
                }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let _ = a.join();
                let _ = b.join();
                if let Some(h) = debug_reader {
                    let _ = h.join();
                }
                let _ = tx.send(Event::Done(status.code().unwrap_or(-1)));
                break;
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = tx.send(Event::Error(0, e.to_string()));
                break;
            }
            _ => (),
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}
#[derive(Clone)]
enum Dialog {
    Open,
    Save(Option<Task>),
    Find,
    Replace,
    Input,
    Quit,
    Help,
    About,
    Windows,
    Close,
    Overwrite(PathBuf, Option<Task>),
}
struct App {
    zorder: Vec<usize>,
    drag: Option<Drag>,
    browser: Option<Browser>,
    list_row: usize,
    close_after_save: bool,
    docs: Vec<Document>,
    active: usize,
    output: Vec<String>,
    status: String,
    dialog: Option<Dialog>,
    entry: String,
    search: String,
    clipboard: String,
    menu: Option<usize>,
    menu_row: usize,
    rx: Receiver<Event>,
    tx: Sender<Event>,
    job: Option<Job>,
    running: bool,
    paused: bool,
    debug_line: Option<usize>,
    watch: String,
    breaks: Vec<usize>,
    options: otp_core::Options,
    output_full: bool,
    quit: bool,
    mouse_down: bool,
    ansi: String,
    in_escape: bool,
}
const MENUS: [(&str, &[&str]); 9] = [
    (
        "File",
        &[
            "New",
            "Open...   F3",
            "Save      F2",
            "Save as...",
            "Quit      Alt+X",
        ],
    ),
    (
        "Edit",
        &[
            "Undo      Ctrl+Z",
            "Redo      Ctrl+Y",
            "Cut       Ctrl+X",
            "Copy      Ctrl+C",
            "Paste     Ctrl+V",
            "Select all",
        ],
    ),
    (
        "Search",
        &["Find...   Ctrl+F", "Find next Ctrl+L", "Replace..."],
    ),
    (
        "Run",
        &[
            "Run       Ctrl+F9",
            "Input...  Enter",
            "Stop      Ctrl+F2",
            "Output    Ctrl+F5",
        ],
    ),
    ("Compile", &["Compile   Alt+F9", "Check syntax"]),
    (
        "Debug",
        &[
            "Trace     F7",
            "Step over F8",
            "Continue",
            "Breakpoint Ctrl+F8",
        ],
    ),
    ("Options", &["Range checking", "Overflow checking"]),
    (
        "Window",
        &[
            "Tile",
            "Cascade",
            "Zoom      F5",
            "Next      F6",
            "Previous Shift+F6",
            "Close     Alt+F3",
            "List...",
        ],
    ),
    ("Help", &["Help      F1", "About OTP"]),
];
impl App {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            zorder: vec![0],
            drag: None,
            browser: None,
            list_row: 0,
            close_after_save: false,
            docs: vec![Document::new(SAMPLE, None)],
            active: 0,
            output: vec![
                "ODIN Turbo Pascal 2.0.0-dev.2 | Native compiler written in Rust".into(),
                "F3 Open  |  Alt+F9 Compile  |  Ctrl+F9 Run  |  F7 Trace".into(),
            ],
            status: "Ready. Development build; see Help for compatibility.".into(),
            dialog: None,
            entry: String::new(),
            search: String::new(),
            clipboard: String::new(),
            menu: None,
            menu_row: 0,
            rx,
            tx,
            job: None,
            running: false,
            paused: false,
            debug_line: None,
            watch: String::new(),
            breaks: vec![],
            options: otp_core::Options::default(),
            output_full: false,
            quit: false,
            mouse_down: false,
            ansi: String::new(),
            in_escape: false,
        }
    }
    fn doc(&self) -> &Document {
        &self.docs[self.active]
    }
    fn doc_mut(&mut self) -> &mut Document {
        self.docs[self.active].follow_cursor = true;
        &mut self.docs[self.active]
    }
    fn open(&mut self, p: PathBuf) {
        match std::fs::read_to_string(&p) {
            Ok(s) => {
                let path = std::fs::canonicalize(&p).unwrap_or(p);
                if let Some(i) = self
                    .docs
                    .iter()
                    .position(|d| d.path.as_ref() == Some(&path))
                {
                    self.focus_document(i);
                } else {
                    self.docs.push(Document::new(&s, Some(path)));
                    self.register_document();
                }
                self.debug_line = None;
                self.breaks.clear();
                self.status = "File opened".into();
            }
            Err(e) => self.status = format!("Open failed: {e}"),
        }
    }
    fn dialog(&mut self, d: Dialog, initial: String) {
        self.menu = None;
        self.drag = None;
        self.browser = None;
        if matches!(d, Dialog::Open | Dialog::Save(_)) {
            let base = self
                .doc()
                .path
                .as_ref()
                .and_then(|p| p.parent())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let (browser, name) = Browser::new(base, &initial);
            self.browser = Some(browser);
            self.entry = name;
        } else {
            self.entry = initial;
        }
        self.list_row = self.active;
        self.dialog = Some(d);
    }
    fn save(&mut self, after: Option<Task>) {
        if let Some(p) = self.doc().path.clone() {
            self.save_to(p, after)
        } else {
            self.dialog(
                Dialog::Save(after),
                std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                    .map(PathBuf::from)
                    .map(|home| {
                        let documents = home.join("Documents");
                        if documents.is_dir() {
                            documents
                        } else {
                            home
                        }
                    })
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("NONAME.PAS")
                    .display()
                    .to_string(),
            )
        }
    }
    fn save_to(&mut self, p: PathBuf, after: Option<Task>) {
        let data = self.doc().text();
        match std::fs::write(&p, data) {
            Ok(()) => {
                self.doc_mut().dirty = false;
                self.doc_mut().path = Some(std::fs::canonicalize(&p).unwrap_or(p));
                self.status = "Saved".into();
                self.dialog = None;
                self.browser = None;
                if self.close_after_save {
                    self.close_after_save = false;
                    self.remove_document();
                }
                if let Some(t) = after {
                    self.start(t);
                }
            }
            Err(e) => {
                self.status = format!("Save failed: {e}");
                if self.dialog.is_none() {
                    self.close_after_save = false;
                }
            }
        }
    }
    fn start(&mut self, t: Task) {
        if self.running {
            if self.paused && matches!(t, Task::Run) {
                self.debug_cmd("CONT");
            } else {
                self.status = "A program is active. Ctrl+F2 stops it.".into();
            }
            return;
        }
        let Some(path) = self.doc().path.clone() else {
            self.save(Some(t));
            return;
        };
        if self.doc().dirty {
            self.save(Some(t));
            return;
        }
        if let Some(job) = self.job.take() {
            let _ = job.handle.join();
        }
        let (tx, rx) = mpsc::channel();
        let events = self.tx.clone();
        let opts = self.options.clone();
        let handle = std::thread::spawn(move || worker(path, t, opts, events, rx));
        self.job = Some(Job { tx, handle });
        self.running = true;
        self.paused = false;
        self.debug_line = None;
        self.output.clear();
        self.output.push(String::new());
        self.watch.clear();
        self.status = "Compiling native executable...".into();
    }
    fn debug_cmd(&mut self, s: &str) {
        if let Some(job) = &self.job {
            let _ = job.tx.send(Control::Debug(s.into()));
        }
        if !s.starts_with("BREAK") {
            self.paused = false;
            self.status = "Program executing...".into();
        }
    }
    fn stop(&mut self) {
        if let Some(j) = &self.job {
            let _ = j.tx.send(Control::Stop);
        }
        self.status = "Stopping program...".into();
    }
    fn append(&mut self, s: &str) {
        for c in s.chars() {
            if self.in_escape {
                self.ansi.push(c);
                if c.is_ascii_alphabetic() {
                    if self.ansi == "[2J" {
                        self.output.clear();
                        self.output.push(String::new());
                    }
                    self.in_escape = false;
                    self.ansi.clear();
                } else if self.ansi.len() > 64 {
                    self.in_escape = false;
                    self.ansi.clear();
                }
                continue;
            }
            if c == '\x1b' {
                self.in_escape = true;
                continue;
            }
            if self.output.is_empty() {
                self.output.push(String::new());
            }
            match c {
                '\n' => self.output.push(String::new()),
                '\r' => (),
                c if !c.is_control() => self.output.last_mut().unwrap().push(c),
                _ => (),
            }
            if self.output.last().unwrap().chars().count() > 96 {
                self.output.push(String::new());
            }
            if self.output.len() > 500 {
                self.output.remove(0);
            }
        }
    }
    fn events(&mut self) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                Event::Output(s) => self.append(&s),
                Event::Built(p) => self.status = format!("Native executable: {}", p.display()),
                Event::Error(line, s) => {
                    self.running = false;
                    self.paused = false;
                    if line > 0 {
                        self.doc_mut().row = (line - 1).min(self.doc().lines.len() - 1);
                        self.doc_mut().col = 0;
                    }
                    self.status = format!("Error: {s}");
                    self.append(&format!("\n{s}\n"));
                }
                Event::Done(n) => {
                    self.running = false;
                    self.paused = false;
                    self.debug_line = None;
                    self.status = format!("Finished. Exit status {n}");
                }
                Event::Paused(line, s) => {
                    let first = self.debug_line.is_none();
                    self.paused = true;
                    self.debug_line = Some(line);
                    self.doc_mut().row = line.saturating_sub(1).min(self.doc().lines.len() - 1);
                    self.doc_mut().col = 0;
                    self.watch = s;
                    self.status = format!(
                        "Paused at line {line}. F7 Trace / F8 Step over / Ctrl+F9 Continue"
                    );
                    if first {
                        for n in self.breaks.clone() {
                            self.debug_cmd(&format!("BREAK {n}"));
                        }
                    }
                }
            }
        }
    }
    fn action(&mut self, label: &str) {
        self.menu = None;
        if self.running
            && [
                "New",
                "Open...   F3",
                "Undo      Ctrl+Z",
                "Redo      Ctrl+Y",
                "Cut       Ctrl+X",
                "Paste     Ctrl+V",
                "Replace...",
                "Next      F6",
                "Previous Shift+F6",
                "Close     Alt+F3",
                "List...",
                "Save as...",
            ]
            .contains(&label)
        {
            self.status = "Stop the current program before changing its source document".into();
            return;
        }
        if self.running && !self.paused && label == "Breakpoint Ctrl+F8" {
            self.status = "Set breakpoints before starting, or while paused".into();
            return;
        }
        match label {
            "New" => {
                self.docs.push(Document::new(
                    "program NewProgram;\n\nbegin\n  WriteLn('ODIN Turbo Pascal');\nend.\n",
                    None,
                ));
                self.register_document();
                self.doc_mut().dirty = true;
            }
            "Open...   F3" => self.dialog(Dialog::Open, String::new()),
            "Save      F2" => self.save(None),
            "Save as..." => self.dialog(
                Dialog::Save(None),
                self.doc()
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or("NONAME.PAS".into()),
            ),
            "Quit      Alt+X" => {
                if self.docs.iter().any(|d| d.dirty) {
                    self.dialog(Dialog::Quit, String::new())
                } else {
                    self.quit = true
                }
            }
            "Undo      Ctrl+Z" => self.doc_mut().undo(),
            "Redo      Ctrl+Y" => self.doc_mut().redo(),
            "Copy      Ctrl+C" => {
                self.clipboard = self.doc().selected();
                self.status = "Copied to OTP's internal clipboard".into();
            }
            "Cut       Ctrl+X" => {
                self.clipboard = self.doc().selected();
                if !self.clipboard.is_empty() {
                    self.doc_mut().backspace(false);
                }
            }
            "Paste     Ctrl+V" => {
                let s = self.clipboard.clone();
                if !s.is_empty() {
                    self.doc_mut().insert(&s);
                }
            }
            "Select all" => {
                let d = self.doc_mut();
                d.anchor = Some((0, 0));
                d.row = d.lines.len() - 1;
                d.col = d.lines[d.row].chars().count();
            }
            "Find...   Ctrl+F" => self.dialog(Dialog::Find, self.search.clone()),
            "Find next Ctrl+L" => {
                let s = self.search.clone();
                self.status = if self.doc_mut().find(&s) {
                    "Match selected".into()
                } else {
                    "Text not found".into()
                };
            }
            "Replace..." => self.dialog(Dialog::Replace, format!("{}|", self.search)),
            "Run       Ctrl+F9" => self.start(Task::Run),
            "Compile   Alt+F9" => self.start(Task::Build),
            "Check syntax" => {
                let src = self.doc().text();
                match otp_core::native::Target::host()
                    .map_err(|e| e.to_string())
                    .and_then(|t| {
                        otp_core::compile(src.as_bytes(), t, &self.options)
                            .map_err(|e| e.to_string())
                    }) {
                    Ok(_) => self.status = "Semantic checks passed for implemented subset".into(),
                    Err(e) => {
                        self.status = format!("Error: {e}");
                        self.append(&format!("\n{e}\n"));
                    }
                }
            }
            "Trace     F7" => {
                if self.paused {
                    self.debug_cmd("STEP")
                } else {
                    self.start(Task::Debug)
                }
            }
            "Step over F8" => {
                if self.paused {
                    self.debug_cmd("NEXT")
                } else {
                    self.start(Task::Debug)
                }
            }
            "Continue" => {
                if self.paused {
                    self.debug_cmd("CONT")
                } else {
                    self.start(Task::Debug)
                }
            }
            "Breakpoint Ctrl+F8" => {
                let n = self.doc().row + 1;
                if let Some(i) = self.breaks.iter().position(|x| *x == n) {
                    self.breaks.remove(i);
                } else {
                    self.breaks.push(n);
                }
                if self.paused {
                    self.debug_cmd(&format!("BREAK {n}"));
                }
                self.status = format!("Breakpoint toggled at line {n}");
            }
            "Input...  Enter" => {
                if self.running {
                    self.dialog(Dialog::Input, String::new())
                } else {
                    self.status = "No running program".into()
                }
            }
            "Stop      Ctrl+F2" => self.stop(),
            "Output    Ctrl+F5" => self.output_full = !self.output_full,
            "Range checking" => {
                self.options.range = !self.options.range;
                self.status = format!("Range checking: {}", self.options.range);
            }
            "Overflow checking" => {
                self.options.overflow = !self.options.overflow;
                self.status = format!("Overflow checking: {}", self.options.overflow);
            }
            "Tile" => self.tile(),
            "Cascade" => self.cascade(),
            "Zoom      F5" => self.zoom(),
            "Close     Alt+F3" => self.request_close(),
            "List..." => self.dialog(Dialog::Windows, String::new()),
            "Next      F6" => self.focus_document((self.active + 1) % self.docs.len()),
            "Previous Shift+F6" => {
                self.focus_document((self.active + self.docs.len() - 1) % self.docs.len())
            }
            "Help      F1" => self.dialog(Dialog::Help, String::new()),
            "About OTP" => self.dialog(Dialog::About, String::new()),
            _ => (),
        }
    }
    fn accept(&mut self) {
        if self.browser.is_some() {
            self.accept_browser();
            return;
        }
        let Some(d) = self.dialog.take() else { return };
        let text = self.entry.clone();
        match d {
            Dialog::Open => self.open(PathBuf::from(text)),
            Dialog::Save(after) => self.save_to(PathBuf::from(text), after),
            Dialog::Find => {
                self.search = text;
                self.action("Find next Ctrl+L");
            }
            Dialog::Replace => {
                if let Some((a, b)) = text.split_once('|') {
                    if !a.is_empty() {
                        let d = self.doc_mut();
                        let src = d.text();
                        let n = src.matches(a).count();
                        if n > 0 {
                            d.checkpoint();
                            d.lines = src.replace(a, b).split('\n').map(str::to_string).collect();
                            d.row = 0;
                            d.col = 0;
                            d.anchor = None;
                        }
                        self.status = format!("Replaced {n} occurrences (case sensitive)");
                    }
                } else {
                    self.status = "Use search|replacement in Replace".into();
                }
            }
            Dialog::Input => {
                if let Some(j) = &self.job {
                    let _ = j.tx.send(Control::Input(text.clone()));
                }
                self.append(&format!("{text}\n"));
            }
            Dialog::Quit => {
                if text == "DISCARD" {
                    self.quit = true
                } else {
                    self.status = "Exit cancelled; F2 saves the active file".into()
                }
            }
            Dialog::Windows => self.focus_document(self.list_row),
            Dialog::Close => match text.to_ascii_uppercase().as_str() {
                "SAVE" | "S" => {
                    self.close_after_save = true;
                    self.save(None);
                }
                "DISCARD" | "D" => self.remove_document(),
                _ => self.status = "Close cancelled".into(),
            },
            Dialog::Overwrite(path, after) => {
                if text.eq_ignore_ascii_case("YES") || text.eq_ignore_ascii_case("Y") {
                    // Retain the prompt if writing fails.
                    self.dialog = Some(Dialog::Overwrite(path.clone(), after));
                    self.save_to(path, after);
                } else {
                    self.close_after_save = false;
                }
            }
            Dialog::Help | Dialog::About => (),
        }
    }
    fn keys(&mut self, keys: Vec<Key>, ctrl: bool, alt: bool, shift: bool) {
        for key in keys {
            if self.dialog.is_some() {
                if self.browser.is_some() {
                    self.browser_key(key, ctrl);
                    continue;
                }
                if matches!(self.dialog, Some(Dialog::Windows)) {
                    match key {
                        Key::Up => self.list_row = self.list_row.saturating_sub(1),
                        Key::Down => self.list_row = (self.list_row + 1).min(self.docs.len() - 1),
                        Key::Home => self.list_row = 0,
                        Key::End => self.list_row = self.docs.len() - 1,
                        Key::Enter => self.accept(),
                        Key::Escape => self.cancel_dialog(),
                        _ => (),
                    }
                    continue;
                }
                match key {
                    Key::Escape => self.cancel_dialog(),
                    Key::Enter => self.accept(),
                    Key::Backspace => {
                        self.entry.pop();
                    }
                    _ => (),
                }
                continue;
            }
            if let Some(m) = self.menu {
                match key {
                    Key::Escape => self.menu = None,
                    Key::Up => {
                        self.menu_row = (self.menu_row + MENUS[m].1.len() - 1) % MENUS[m].1.len()
                    }
                    Key::Down => self.menu_row = (self.menu_row + 1) % MENUS[m].1.len(),
                    Key::Left => {
                        self.menu = Some((m + 8) % 9);
                        self.menu_row = 0;
                    }
                    Key::Right => {
                        self.menu = Some((m + 1) % 9);
                        self.menu_row = 0;
                    }
                    Key::Enter => self.action(MENUS[m].1[self.menu_row]),
                    _ => (),
                }
                continue;
            }
            let action = match key {
                Key::F1 => Some("Help      F1"),
                Key::F2 if ctrl => Some("Stop      Ctrl+F2"),
                Key::F2 => Some("Save      F2"),
                Key::F3 if alt => Some("Close     Alt+F3"),
                Key::F3 => Some("Open...   F3"),
                Key::F5 if ctrl => Some("Output    Ctrl+F5"),
                Key::F5 => Some("Zoom      F5"),
                Key::F6 if shift => Some("Previous Shift+F6"),
                Key::F6 => Some("Next      F6"),
                Key::F7 => Some("Trace     F7"),
                Key::F8 if ctrl => Some("Breakpoint Ctrl+F8"),
                Key::F8 => Some("Step over F8"),
                Key::F9 if ctrl => Some("Run       Ctrl+F9"),
                Key::F9 if alt => Some("Compile   Alt+F9"),
                Key::X if alt => Some("Quit      Alt+X"),
                Key::Z if ctrl => Some("Undo      Ctrl+Z"),
                Key::Y if ctrl => Some("Redo      Ctrl+Y"),
                Key::C if ctrl => Some("Copy      Ctrl+C"),
                Key::X if ctrl => Some("Cut       Ctrl+X"),
                Key::V if ctrl => Some("Paste     Ctrl+V"),
                Key::A if ctrl => Some("Select all"),
                Key::F if ctrl => Some("Find...   Ctrl+F"),
                Key::L if ctrl => Some("Find next Ctrl+L"),
                Key::Enter if self.running => Some("Input...  Enter"),
                _ => None,
            };
            if let Some(a) = action {
                self.action(a);
                continue;
            }
            if key == Key::F10 {
                self.menu = Some(0);
                self.menu_row = 0;
                continue;
            }
            if alt {
                let k = match key {
                    Key::F => Some(0),
                    Key::E => Some(1),
                    Key::S => Some(2),
                    Key::R => Some(3),
                    Key::C => Some(4),
                    Key::D => Some(5),
                    Key::O => Some(6),
                    Key::W => Some(7),
                    Key::H => Some(8),
                    _ => None,
                };
                self.menu = k;
                self.menu_row = 0;
                continue;
            }
            if ctrl {
                continue;
            }
            // Source stays fixed while a process is active, so debugger line numbers remain valid.
            if self.running {
                continue;
            }
            self.doc_mut().follow_cursor = true;
            let page_height = self.doc().rect.h.saturating_sub(2);
            match key {
                Key::Enter => {
                    let indent: String = self.doc().lines[self.doc().row]
                        .chars()
                        .take_while(|c| *c == ' ')
                        .collect();
                    self.doc_mut().insert(&format!("\n{indent}"));
                }
                Key::Tab => self.doc_mut().insert("  "),
                Key::Backspace => self.doc_mut().backspace(false),
                Key::Delete => self.doc_mut().backspace(true),
                Key::Escape => self.doc_mut().anchor = None,
                _ => self.doc_mut().movement(key, shift, page_height),
            }
        }
    }
    fn chars(&mut self, chars: Vec<char>, suppress: bool) {
        if suppress {
            return;
        }
        for c in chars {
            if let Some(d) = &self.dialog {
                if !matches!(d, Dialog::Help | Dialog::About | Dialog::Windows)
                    && self.browser.as_ref().map(|b| !b.list_focus).unwrap_or(true)
                {
                    self.entry.push(c);
                }
            } else if self.menu.is_none() && !self.running {
                self.doc_mut().follow_cursor = true;
                self.doc_mut().insert(&c.to_string());
            }
        }
    }
    fn render(&mut self, canvas: &mut Canvas) {
        canvas.pixels.fill(BLUE);
        canvas.bar(
            0,
            " ODIN   File   Edit   Search   Run   Compile   Debug   Options   Window   Help",
            BLACK,
            GRAY,
        );
        if !self.output_full {
            self.render_desktop(canvas);
        }
        let d = self.doc();
        let oy = if self.output_full { 1 } else { 23 };
        let oh = ROWS - oy - 2;
        canvas.box_at(
            0,
            oy,
            100,
            oh,
            if self.paused {
                " Output / Variables "
            } else {
                " Program output "
            },
            BLACK,
            GRAY,
        );
        let usable = oh.saturating_sub(if self.paused { 4 } else { 2 });
        let start = self.output.len().saturating_sub(usable);
        for (i, s) in self.output[start..].iter().take(usable).enumerate() {
            canvas.text(2, oy + 1 + i, s, BLACK, GRAY);
        }
        if self.paused {
            canvas.text(
                2,
                oy + oh - 3,
                &self.watch.chars().take(95).collect::<String>(),
                BLUE,
                GRAY,
            );
            canvas.text(
                2,
                oy + oh - 2,
                "F7 Trace  F8 Step over  Ctrl+F8 Breakpoint  Ctrl+F9 Continue",
                BLUE,
                GRAY,
            );
        }
        canvas.bar(
            33,
            &format!("{}:{}  {}", d.row + 1, d.col + 1, self.status),
            WHITE,
            CYAN,
        );
        canvas.bar(
            34,
            "F1 Help  F2 Save  F3 Open  F5 Zoom  F6 Next  Ctrl+F5 Output  F10 Menu  Alt+X Exit",
            BLACK,
            GRAY,
        );
        if let Some(m) = self.menu {
            let starts = [9, 16, 23, 32, 38, 48, 56, 66, 75];
            let x = starts[m].min(COLS - 26);
            canvas.box_at(x, 1, 26, MENUS[m].1.len() + 2, MENUS[m].0, BLACK, GRAY);
            for (i, item) in MENUS[m].1.iter().enumerate() {
                canvas.text(
                    x + 1,
                    i + 2,
                    &format!("{item:24}"),
                    if i == self.menu_row { WHITE } else { BLACK },
                    if i == self.menu_row { BLUE } else { GRAY },
                );
            }
        }
        if self.browser.is_some() {
            self.render_browser(canvas);
            return;
        }
        if matches!(self.dialog, Some(Dialog::Windows)) {
            self.render_window_list(canvas);
            return;
        }
        if let Some(dialog) = &self.dialog {
            if matches!(dialog, Dialog::Help | Dialog::About) {
                canvas.box_at(
                    9,
                    5,
                    82,
                    25,
                    if matches!(dialog, Dialog::About) {
                        "About ODIN Turbo Pascal"
                    } else {
                        "OTP Help"
                    },
                    BLACK,
                    GRAY,
                );
                let lines = if matches!(dialog, Dialog::About) {
                    vec![
                        "ODIN TURBO PASCAL 2.0.0-dev.2",
                        "Independent Rust implementation. GPL-3.0-only.",
                        "",
                        "Reference: Turbo Pascal 7.0 DOS (1992).",
                        "This development milestone is NOT a complete TP7 clone.",
                        "",
                        "Native assembly backends: x86_64 and Apple ARM64.",
                        "Rust runtime; no Free Pascal compiler dependency.",
                        "Native linking needs cc/clang and the bundled runtime.",
                        "",
                        "Original Odin artwork: assets/branding/",
                        "Compiler and compatibility notes: docs/COMPATIBILITY.md",
                        "",
                        "Enter or Esc closes this window.",
                    ]
                } else {
                    vec![
                        "FILE: F3 browses files; F2 saves. F6 changes document.",
                        "EDIT: arrows, Home/End, PgUp/PgDn, Shift selection.",
                        "Ctrl+Z/Y undo/redo. Ctrl+A selects all.",
                        "Ctrl+C/X/V use OTP's internal clipboard.",
                        "SEARCH: Ctrl+F find; Ctrl+L next. Replace: old|new.",
                        "BUILD: Alt+F9 compiles a native program beside source.",
                        "RUN: Ctrl+F9. Enter sends input. Ctrl+F2 stops.",
                        "DEBUG: F7 starts and traces; F8 steps over procedures.",
                        "Ctrl+F8 toggles a breakpoint; Ctrl+F9 continues.",
                        "Visible scalar variables appear below the output.",
                        "Ctrl+F5 expands output. F10 or Alt+letter opens a menu.",
                        "",
                        "Implemented: scalar types, strings, arrays, records,",
                        "procedures/functions, loops, CASE, text file I/O.",
                        "WINDOW: Tile / Cascade / F5 Zoom / Alt+F3 Close / List.",
                        "Drag title to move; drag bottom-right corner to resize.",
                        "Not yet: custom units, objects, pointers, sets, BGI,",
                        "Turbo Vision, DOS machine services, full TP7 behavior.",
                        "Output panel is a transcript, not a DOS terminal emulator.",
                        "See README.md and docs/COMPATIBILITY.md for details.",
                        "",
                        "Enter or Esc closes Help.",
                    ]
                };
                for (i, s) in lines.iter().enumerate() {
                    canvas.text(12, 7 + i, s, BLACK, GRAY);
                }
                if matches!(dialog, Dialog::About) {
                    canvas.logo(690, 350, 170);
                }
            } else {
                let (title, hint) = match dialog {
                    Dialog::Open => (
                        "Open Pascal file",
                        "Enter a file path; Enter opens, Esc cancels.",
                    ),
                    Dialog::Save(_) => (
                        "Save Pascal file",
                        "Enter a path; existing file at this path will be replaced.",
                    ),
                    Dialog::Find => (
                        "Find text",
                        "Case insensitive. Enter searches; Ctrl+L repeats.",
                    ),
                    Dialog::Replace => ("Replace all", "Enter search|replacement. Case sensitive."),
                    Dialog::Input => (
                        "Program input",
                        "Enter sends a line to the running program.",
                    ),
                    Dialog::Close => (
                        "Close modified document",
                        "Type SAVE or DISCARD, then Enter. Esc cancels.",
                    ),
                    Dialog::Overwrite(_, _) => (
                        "Replace existing file?",
                        "Type YES then Enter to overwrite. Esc cancels.",
                    ),
                    Dialog::Quit => (
                        "Unsaved documents",
                        "Type DISCARD to quit without saving, or Esc to return.",
                    ),
                    _ => ("", ""),
                };
                canvas.box_at(10, 10, 80, 9, title, BLACK, GRAY);
                canvas.text(12, 12, hint, BLACK, GRAY);
                let subject = match dialog {
                    Dialog::Overwrite(path, _) => path.display().to_string(),
                    Dialog::Close => self.doc().name(),
                    _ => String::new(),
                };
                canvas.text(
                    12,
                    13,
                    &subject.chars().take(75).collect::<String>(),
                    BLUE,
                    GRAY,
                );
                canvas.rect(12 * CW, 14 * CH, 75 * CW, CH, BLUE);
                let view: String = self
                    .entry
                    .chars()
                    .rev()
                    .take(73)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                canvas.text(13, 14, &view, YELLOW, BLUE);
                canvas.text(12, 17, "[ Enter: OK ]        [ Esc: Cancel ]", BLACK, GRAY);
            }
        }
    }
}
fn syntax(line: &str) -> Vec<u32> {
    let chars: Vec<_> = line.chars().collect();
    let mut colors = vec![YELLOW; chars.len()];
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\'' {
            let start = i;
            i += 1;
            while i < chars.len() {
                if chars[i] == '\'' {
                    i += 1;
                    if i < chars.len() && chars[i] == '\'' {
                        i += 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            colors[start..i].fill(0x55ffff);
        } else if chars[i] == '{' || (chars[i] == '/' && chars.get(i + 1) == Some(&'/')) {
            colors[i..].fill(GRAY);
            break;
        } else if chars[i].is_ascii_alphabetic() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let s = chars[start..i]
                .iter()
                .collect::<String>()
                .to_ascii_lowercase();
            if [
                "program",
                "uses",
                "var",
                "const",
                "type",
                "begin",
                "end",
                "if",
                "then",
                "else",
                "for",
                "to",
                "downto",
                "do",
                "while",
                "repeat",
                "until",
                "procedure",
                "function",
                "array",
                "of",
                "record",
                "case",
                "div",
                "mod",
                "and",
                "or",
                "not",
                "xor",
                "true",
                "false",
            ]
            .contains(&s.as_str())
            {
                colors[start..i].fill(WHITE);
            }
        } else {
            i += 1
        }
    }
    colors
}
fn main() {
    let mut app = App::new();
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--version") {
        println!("ODIN Turbo Pascal IDE 2.0.0-dev.2");
        return;
    }
    if args.first().map(String::as_str) == Some("--screenshot") {
        if args.iter().any(|a| a == "--tile" || a == "--cascade") {
            app.action("New");
            app.action("New");
            if args.iter().any(|a| a == "--tile") {
                app.tile();
            } else {
                app.cascade();
            }
        }
        if args.iter().any(|a| a == "--files") {
            app.dialog(Dialog::Open, "examples".into());
        }
        if args.iter().any(|a| a == "--windows") {
            app.dialog(Dialog::Windows, String::new());
        }
        if args.iter().any(|a| a == "--about") {
            app.dialog = Some(Dialog::About);
        }
        let mut canvas = Canvas::new();
        app.render(&mut canvas);
        if let Err(e) = canvas.ppm(Path::new(
            args.get(1).map(String::as_str).unwrap_or("otp.ppm"),
        )) {
            eprintln!("{e}");
            std::process::exit(1)
        }
        return;
    }
    if let Some(p) = args.first() {
        app.open(PathBuf::from(p));
    }
    let mut window = match Window::new(
        "ODIN Turbo Pascal 2.0 | Rust development edition",
        W,
        H,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Cannot open desktop window: {e}\nUse otpc for command-line compilation.");
            std::process::exit(1)
        }
    };
    window.set_target_fps(60);
    let (tx, rx) = mpsc::channel();
    window.set_input_callback(Box::new(Chars(tx)));
    let mut canvas = Canvas::new();
    while window.is_open() && !app.quit {
        app.events();
        let ctrl = window.is_key_down(Key::LeftCtrl)
            || window.is_key_down(Key::RightCtrl)
            || window.is_key_down(Key::LeftSuper)
            || window.is_key_down(Key::RightSuper);
        let alt = window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::RightAlt);
        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let keys = window.get_keys_pressed(KeyRepeat::Yes);
        app.keys(keys, ctrl, alt, shift);
        app.chars(rx.try_iter().collect(), ctrl || alt);
        let down = window.get_mouse_down(MouseButton::Left);
        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Discard) {
            let (size_x, size_y) = window.get_size();
            let cx = (x as usize * COLS / size_x.max(1)).min(COLS - 1);
            let cy = (y as usize * ROWS / size_y.max(1)).min(ROWS - 1);
            app.pointer(cx, cy, down, shift);
            if let Some((dx, dy)) = window.get_scroll_wheel() {
                app.wheel(cx, cy, dx, dy, shift);
            }
        } else if !down {
            app.drag = None;
        }
        app.mouse_down = down;
        app.render(&mut canvas);
        if window.update_with_buffer(&canvas.pixels, W, H).is_err() {
            break;
        }
    }
    if let Some(j) = app.job.take() {
        let _ = j.tx.send(Control::Stop);
        let _ = j.handle.join();
    }
    // Window-manager close preserves unsaved documents in separate recovery files.
    for (i, d) in app.docs.iter().enumerate() {
        if d.dirty && !app.quit {
            let root = std::env::temp_dir().join("odin-turbo-pascal-recovery");
            let _ = std::fs::create_dir_all(&root);
            let p = root.join(format!("{}-{i}-{}", std::process::id(), d.name()));
            if std::fs::write(&p, d.text()).is_ok() {
                eprintln!("Recovered unsaved document: {}", p.display());
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edit_undo_unicode_selection() {
        let mut d = Document::new("begin\n  writeln('é');\nend.", None);
        d.row = 1;
        d.col = 2;
        d.insert("X\n");
        assert_eq!(d.lines[1], "  X");
        d.undo();
        assert_eq!(d.lines[1], "  writeln('é');");
        d.redo();
        assert_eq!(d.lines[1], "  X");
        d.anchor = Some((0, 0));
        d.row = d.lines.len() - 1;
        d.col = 4;
        d.insert("done");
        assert_eq!(d.text(), "done");
        d.undo();
        assert!(d.text().starts_with("begin"));
    }
    #[test]
    fn search_wrap() {
        let mut d = Document::new("hello\nHELLO", None);
        assert!(d.find("hello"));
        assert_eq!(d.row, 1);
        assert!(d.find("hello"));
        assert_eq!(d.row, 0);
    }
}
