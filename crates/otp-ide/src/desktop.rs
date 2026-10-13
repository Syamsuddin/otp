// SPDX-License-Identifier: GPL-3.0-only
//! Internal editor windows and portable file browser, sharing the live event handlers.
use super::*;
const DESKTOP_BOTTOM: usize = 23;
const MIN_W: usize = 24;
const MIN_H: usize = 6;
const LIST_ROWS: usize = 14;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}
impl Rect {
    pub fn desktop() -> Self {
        Self {
            x: 0,
            y: 1,
            w: COLS,
            h: 22,
        }
    }
    fn contains(self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
    fn bounded(self) -> Self {
        let w = self.w.clamp(MIN_W, COLS);
        let h = self.h.clamp(MIN_H, DESKTOP_BOTTOM - 1);
        Self {
            x: self.x.min(COLS - w),
            y: self.y.clamp(1, DESKTOP_BOTTOM - h),
            w,
            h,
        }
    }
    fn cols(self) -> usize {
        self.w.saturating_sub(8)
    }
    fn rows(self) -> usize {
        self.h.saturating_sub(2)
    }
}
#[derive(Clone, Copy)]
pub(super) enum Drag {
    Move(usize, usize, usize),
    Resize(usize),
    Vertical(usize),
    Horizontal(usize),
}
struct Entry {
    path: PathBuf,
    label: String,
    dir: bool,
}
pub(super) struct Browser {
    cwd: PathBuf,
    entries: Vec<Entry>,
    selected: usize,
    top: usize,
    pub list_focus: bool,
    all_files: bool,
    error: String,
    last_click: Option<(usize, Instant)>,
}
impl Browser {
    pub fn new(base: PathBuf, initial: &str) -> (Self, String) {
        let p = if initial.is_empty() {
            base.clone()
        } else {
            let p = PathBuf::from(initial);
            if p.is_absolute() {
                p
            } else {
                base.join(p)
            }
        };
        let (dir, name) = if p.is_dir() {
            (p, String::new())
        } else {
            (
                p.parent().unwrap_or(&base).to_path_buf(),
                p.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            )
        };
        let mut b = Self {
            cwd: base,
            entries: vec![],
            selected: 0,
            top: 0,
            list_focus: false,
            all_files: false,
            error: String::new(),
            last_click: None,
        };
        b.load(dir);
        (b, name)
    }
    fn load(&mut self, dir: PathBuf) {
        let result = (|| -> std::io::Result<(PathBuf, Vec<Entry>)> {
            let dir = std::fs::canonicalize(dir)?;
            let mut entries = vec![];
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                let is_dir = path.is_dir();
                let ext = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase();
                if is_dir || self.all_files || ["pas", "pp", "inc"].contains(&ext.as_str()) {
                    entries.push(Entry {
                        label: entry.file_name().to_string_lossy().into_owned(),
                        path,
                        dir: is_dir,
                    });
                }
            }
            entries.sort_by(|a, b| {
                b.dir
                    .cmp(&a.dir)
                    .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
            });
            if let Some(parent) = dir.parent() {
                entries.insert(
                    0,
                    Entry {
                        path: parent.to_path_buf(),
                        label: "..".into(),
                        dir: true,
                    },
                );
            }
            Ok((dir, entries))
        })();
        match result {
            Ok((dir, entries)) => {
                self.cwd = dir;
                self.entries = entries;
                self.selected = 0;
                self.top = 0;
                self.error.clear();
                self.last_click = None;
            }
            Err(e) => self.error = format!("Cannot read folder: {e}"),
        }
    }
    fn reveal(&mut self) {
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        if self.selected < self.top {
            self.top = self.selected;
        }
        if self.selected >= self.top + LIST_ROWS {
            self.top = self.selected + 1 - LIST_ROWS;
        }
    }
    fn path(&self, text: &str) -> PathBuf {
        let p = PathBuf::from(text);
        if p.is_absolute() {
            p
        } else {
            self.cwd.join(p)
        }
    }
}
fn clipped(s: &str, width: usize) -> String {
    s.chars().take(width).collect()
}
fn tail(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len <= width {
        s.into()
    } else {
        format!("...{}", s.chars().skip(len - width + 3).collect::<String>())
    }
}
impl App {
    pub(super) fn register_document(&mut self) {
        let i = self.docs.len() - 1;
        let offset = (i % 5) * 3;
        self.docs[i].rect = Rect {
            x: offset,
            y: 1 + i % 5,
            w: COLS - offset,
            h: 22 - i % 5,
        };
        self.zorder.push(i);
        self.focus_document(i);
    }
    pub(super) fn focus_document(&mut self, i: usize) {
        if i >= self.docs.len() {
            return;
        }
        if self.running && i != self.active {
            self.status = "Stop the program before changing documents".into();
            return;
        }
        if i != self.active {
            self.breaks.clear();
            self.debug_line = None;
        }
        self.active = i;
        self.zorder.retain(|n| *n != i);
        self.zorder.push(i);
        self.output_full = false;
        self.doc_mut().follow_cursor = true;
    }
    pub(super) fn tile(&mut self) {
        self.output_full = false;
        let count = self.docs.len();
        // At most twelve usable tiles per page. F6/List reveals another page.
        let start = self.active / 12 * 12;
        let count = count.saturating_sub(start).min(12);
        let cols = if count <= 1 {
            1
        } else if count <= 6 {
            2
        } else {
            4
        };
        let rows = (count + cols - 1) / cols;
        for i in 0..count {
            let x = (i % cols) * COLS / cols;
            let right = (i % cols + 1) * COLS / cols;
            let y = 1 + (i / cols) * 22 / rows;
            let bottom = 1 + (i / cols + 1) * 22 / rows;
            let d = &mut self.docs[start + i];
            d.rect = Rect {
                x,
                y,
                w: right - x,
                h: bottom - y,
            };
            d.restore_rect = None;
            d.follow_cursor = true;
        }
        // Other documents are underneath this page, never made inaccessible.
        self.zorder.retain(|i| *i < start || *i >= start + count);
        self.zorder.extend(start..start + count);
        self.focus_document(self.active);
        self.status = if self.docs.len() > 12 {
            "Tile shows 12 documents per group; F6/List selects others".into()
        } else {
            "Windows tiled".into()
        };
    }
    pub(super) fn cascade(&mut self) {
        self.output_full = false;
        for (i, d) in self.docs.iter_mut().enumerate() {
            let n = i % 7;
            d.rect = Rect {
                x: n * 3,
                y: 1 + n,
                w: COLS - n * 3,
                h: 22 - n,
            };
            d.restore_rect = None;
            d.follow_cursor = true;
        }
        self.zorder = (0..self.docs.len()).collect();
        self.focus_document(self.active);
        self.status = "Windows cascaded".into();
    }
    pub(super) fn zoom(&mut self) {
        self.output_full = false;
        let d = self.doc_mut();
        if let Some(r) = d.restore_rect.take() {
            d.rect = r;
        } else {
            d.restore_rect = Some(d.rect);
            d.rect = Rect::desktop();
        }
        d.follow_cursor = true;
    }
    pub(super) fn request_close(&mut self) {
        if self.running {
            self.status = "Stop the program before closing its document".into();
            return;
        }
        if self.doc().dirty {
            self.dialog(Dialog::Close, String::new());
        } else {
            self.remove_document();
        }
    }
    pub(super) fn remove_document(&mut self) {
        let i = self.active;
        self.docs.remove(i);
        self.zorder.retain(|n| *n != i);
        for n in &mut self.zorder {
            if *n > i {
                *n -= 1;
            }
        }
        if self.docs.is_empty() {
            self.docs.push(Document::new("", None));
            self.zorder = vec![0];
        }
        self.active = *self.zorder.last().unwrap_or(&0);
        self.breaks.clear();
        self.debug_line = None;
        self.drag = None;
        self.close_after_save = false;
        self.status = "Document closed".into();
    }
    pub(super) fn cancel_dialog(&mut self) {
        self.dialog = None;
        self.browser = None;
        self.close_after_save = false;
    }
    pub(super) fn render_desktop(&mut self, canvas: &mut Canvas) {
        for i in self.zorder.clone() {
            let active = i == self.active;
            let d = &mut self.docs[i];
            let r = d.rect;
            let rows = r.rows();
            let cols = r.cols();
            if d.follow_cursor {
                if d.row < d.top {
                    d.top = d.row;
                }
                if d.row >= d.top + rows {
                    d.top = d.row + 1 - rows;
                }
                if d.col < d.left {
                    d.left = d.col;
                }
                if d.col >= d.left + cols {
                    d.left = d.col + 1 - cols;
                }
                d.follow_cursor = false;
            }
            let title_color = if active { BLUE } else { BLACK };
            canvas.box_at(
                r.x,
                r.y,
                r.w,
                r.h,
                "",
                if active { WHITE } else { CYAN },
                BLUE,
            );
            canvas.rect(r.x * CW, r.y * CH, r.w * CW, CH, GRAY);
            canvas.text(r.x, r.y, "[X]", title_color, GRAY);
            let title = format!(
                "{}{} [{}]",
                d.name(),
                if d.dirty { " *" } else { "" },
                i + 1
            );
            let title = clipped(&title, r.w - 8);
            let title_x = r.x + 4 + (r.w - 8 - title.chars().count()) / 2;
            canvas.text(title_x, r.y, &title, title_color, GRAY);
            canvas.text(
                r.x + r.w - 3,
                r.y,
                if d.restore_rect.is_some() {
                    "[-]"
                } else {
                    "[+]"
                },
                title_color,
                GRAY,
            );
            let selected = d.selection();
            for row in 0..rows {
                let n = d.top + row;
                if n >= d.lines.len() {
                    break;
                }
                let y = r.y + 1 + row;
                let mark = if active && self.debug_line == Some(n + 1) {
                    ">"
                } else if active && self.breaks.contains(&(n + 1)) {
                    "*"
                } else {
                    " "
                };
                canvas.text(
                    r.x + 1,
                    y,
                    &clipped(&format!("{mark}{:>4} ", n + 1), 6),
                    CYAN,
                    BLUE,
                );
                let colors = syntax(&d.lines[n]);
                for (j, c) in d.lines[n].chars().enumerate().skip(d.left).take(cols) {
                    let sel = selected
                        .map(|(a, b)| (n, j) >= a && (n, j) < b)
                        .unwrap_or(false);
                    let bg = if sel {
                        CYAN
                    } else if active && self.debug_line == Some(n + 1) {
                        0x555555
                    } else if active && self.breaks.contains(&(n + 1)) {
                        RED
                    } else {
                        BLUE
                    };
                    canvas.cell(
                        r.x + 7 + j - d.left,
                        y,
                        c,
                        if sel { WHITE } else { colors[j] },
                        bg,
                    );
                }
                if active
                    && d.row == n
                    && d.col >= d.left
                    && d.col < d.left + cols
                    && !self.running
                    && self.dialog.is_none()
                {
                    canvas.rect(
                        (r.x + 7 + d.col - d.left) * CW,
                        y * CH + CH - 2,
                        CW,
                        2,
                        WHITE,
                    );
                }
            }
            let max_top = d.lines.len().saturating_sub(rows);
            let max_left = d
                .lines
                .iter()
                .map(|s| s.chars().count())
                .max()
                .unwrap_or(0)
                .saturating_sub(cols - 1);
            let vx = r.x + r.w - 1;
            let hy = r.y + r.h - 1;
            for yy in r.y + 1..hy {
                canvas.cell(vx, yy, '|', BLUE, CYAN);
            }
            canvas.cell(vx, r.y + 1, '^', BLUE, CYAN);
            canvas.cell(vx, hy - 1, 'v', BLUE, CYAN);
            let vt = rows.saturating_sub(3);
            canvas.cell(
                vx,
                r.y + 2
                    + if max_top == 0 {
                        0
                    } else {
                        d.top.min(max_top) * vt / max_top
                    },
                '#',
                WHITE,
                BLUE,
            );
            for xx in r.x + 1..vx {
                canvas.cell(xx, hy, '-', BLUE, CYAN);
            }
            canvas.cell(r.x + 1, hy, '<', BLUE, CYAN);
            canvas.cell(vx - 1, hy, '>', BLUE, CYAN);
            let ht = r.w.saturating_sub(6);
            canvas.cell(
                r.x + 2
                    + if max_left == 0 {
                        0
                    } else {
                        d.left.min(max_left) * ht / max_left
                    },
                hy,
                '#',
                WHITE,
                BLUE,
            );
            canvas.cell(vx, hy, '/', WHITE, GRAY);
        }
    }
    pub(super) fn scroll_to_pointer(&mut self, i: usize, x: usize, y: usize, vertical: bool) {
        let d = &mut self.docs[i];
        let r = d.rect;
        if vertical {
            let max = d.lines.len().saturating_sub(r.rows());
            d.top = y.saturating_sub(r.y + 2).min(r.rows() - 3) * max / (r.rows() - 3).max(1);
        } else {
            let max = d
                .lines
                .iter()
                .map(|s| s.chars().count())
                .max()
                .unwrap_or(0)
                .saturating_sub(r.cols() - 1);
            d.left = x.saturating_sub(r.x + 2).min(r.w - 6) * max / (r.w - 6);
        }
        d.follow_cursor = false;
    }
    pub(super) fn pointer(&mut self, x: usize, y: usize, down: bool, shift: bool) {
        if !down {
            self.drag = None;
            return;
        }
        if self.mouse_down {
            if let Some(drag) = self.drag {
                match drag {
                    Drag::Move(i, dx, dy) => {
                        let d = &mut self.docs[i];
                        d.rect = Rect {
                            x: x.saturating_sub(dx),
                            y: y.saturating_sub(dy),
                            ..d.rect
                        }
                        .bounded();
                        d.restore_rect = None;
                    }
                    Drag::Resize(i) => {
                        let d = &mut self.docs[i];
                        d.rect = Rect {
                            w: (x + 1)
                                .saturating_sub(d.rect.x)
                                .clamp(MIN_W, COLS - d.rect.x),
                            h: (y + 1)
                                .saturating_sub(d.rect.y)
                                .clamp(MIN_H, DESKTOP_BOTTOM - d.rect.y),
                            ..d.rect
                        };
                        d.restore_rect = None;
                        d.follow_cursor = true;
                    }
                    Drag::Vertical(i) => self.scroll_to_pointer(i, x, y, true),
                    Drag::Horizontal(i) => self.scroll_to_pointer(i, x, y, false),
                }
            }
            return;
        }
        if self.browser.is_some() {
            self.browser_click(x, y);
            return;
        }
        if matches!(self.dialog, Some(Dialog::Windows)) {
            let start = self.list_row.saturating_sub(LIST_ROWS - 1);
            if (12..88).contains(&x) && (7..7 + LIST_ROWS).contains(&y) {
                let i = start + y - 7;
                if i < self.docs.len() {
                    self.list_row = i;
                    self.accept();
                }
            } else if y == 24 {
                self.cancel_dialog();
            }
            return;
        }
        if self.dialog.is_some() {
            if y == 17 && (12..28).contains(&x) {
                self.accept();
            } else if y == 17 && (36..55).contains(&x) {
                self.cancel_dialog();
            }
            return;
        }
        if y == 0 {
            let starts = [9, 16, 23, 32, 38, 48, 56, 66, 75];
            self.menu = starts.iter().rposition(|s| x >= *s);
            self.menu_row = 0;
            return;
        }
        if let Some(m) = self.menu {
            let starts = [9, 16, 23, 32, 38, 48, 56, 66, 75];
            let mx = starts[m].min(COLS - 26);
            if x >= mx && x < mx + 26 && y >= 2 && y < 2 + MENUS[m].1.len() {
                self.action(MENUS[m].1[y - 2]);
            } else {
                self.menu = None;
            }
            return;
        }
        if self.output_full {
            return;
        }
        let Some(i) = self
            .zorder
            .iter()
            .rev()
            .find(|i| self.docs[**i].rect.contains(x, y))
            .copied()
        else {
            return;
        };
        if self.running && i != self.active {
            self.status = "Stop the program before changing documents".into();
            return;
        }
        // Preserve manual scroll offsets when clicking the already active window.
        if i != self.active {
            self.focus_document(i);
        }
        let r = self.docs[i].rect;
        if y == r.y {
            if x < r.x + 3 {
                self.request_close();
            } else if x >= r.x + r.w - 3 {
                self.zoom();
            } else {
                self.drag = Some(Drag::Move(i, x - r.x, y - r.y));
            }
        } else if x == r.x + r.w - 1 && y == r.y + r.h - 1 {
            self.drag = Some(Drag::Resize(i));
        } else if x == r.x + r.w - 1 {
            let d = &mut self.docs[i];
            if y == r.y + 1 {
                d.top = d.top.saturating_sub(1);
                d.follow_cursor = false;
            } else if y == r.y + r.h - 2 {
                d.top = (d.top + 1).min(d.lines.len().saturating_sub(r.rows()));
                d.follow_cursor = false;
            } else {
                self.drag = Some(Drag::Vertical(i));
                self.scroll_to_pointer(i, x, y, true);
            }
        } else if y == r.y + r.h - 1 {
            let d = &mut self.docs[i];
            if x == r.x + 1 {
                d.left = d.left.saturating_sub(1);
                d.follow_cursor = false;
            } else if x == r.x + r.w - 2 {
                let max = d
                    .lines
                    .iter()
                    .map(|s| s.chars().count())
                    .max()
                    .unwrap_or(0)
                    .saturating_sub(r.cols() - 1);
                d.left = (d.left + 1).min(max);
                d.follow_cursor = false;
            } else {
                self.drag = Some(Drag::Horizontal(i));
                self.scroll_to_pointer(i, x, y, false);
            }
        } else if x >= r.x + 7 && !self.running {
            let d = &mut self.docs[i];
            let old = (d.row, d.col);
            d.row = (d.top + y - r.y - 1).min(d.lines.len() - 1);
            d.col = (d.left + x - r.x - 7).min(d.lines[d.row].chars().count());
            if shift {
                if d.anchor.is_none() {
                    d.anchor = Some(old);
                }
            } else {
                d.anchor = None;
            }
            d.follow_cursor = true;
        }
    }
    pub(super) fn wheel(&mut self, x: usize, y: usize, dx: f32, dy: f32, shift: bool) {
        if let Some(b) = self.browser.as_mut() {
            let n = (dy.abs() * 3.0).ceil() as usize;
            if dy > 0.0 {
                b.selected = b.selected.saturating_sub(n);
            } else {
                b.selected = (b.selected + n).min(b.entries.len().saturating_sub(1));
            }
            b.list_focus = true;
            b.reveal();
            return;
        }
        if self.dialog.is_some() || self.menu.is_some() || self.output_full {
            return;
        }
        let Some(i) = self
            .zorder
            .iter()
            .rev()
            .find(|i| self.docs[**i].rect.contains(x, y))
            .copied()
        else {
            return;
        };
        let d = &mut self.docs[i];
        let horizontal = shift || dx != 0.0;
        let delta = if shift {
            dy
        } else if horizontal {
            dx
        } else {
            dy
        };
        let max = if horizontal {
            d.lines
                .iter()
                .map(|s| s.chars().count())
                .max()
                .unwrap_or(0)
                .saturating_sub(d.rect.cols() - 1)
        } else {
            d.lines.len().saturating_sub(d.rect.rows())
        };
        let offset = if horizontal { &mut d.left } else { &mut d.top };
        let n = (delta.abs() * 3.0).ceil() as usize;
        *offset = if delta > 0.0 {
            offset.saturating_sub(n)
        } else {
            (*offset + n).min(max)
        };
        d.follow_cursor = false;
    }
    pub(super) fn render_window_list(&self, c: &mut Canvas) {
        c.box_at(10, 4, 80, 23, "Windows", BLACK, GRAY);
        c.text(
            12,
            5,
            "Select with Up/Down and Enter, or click a document.",
            BLACK,
            GRAY,
        );
        let start = self.list_row.saturating_sub(LIST_ROWS - 1);
        for (row, (i, d)) in self
            .docs
            .iter()
            .enumerate()
            .skip(start)
            .take(LIST_ROWS)
            .enumerate()
        {
            let selected = i == self.list_row;
            let name = d
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| d.name());
            let text = clipped(
                &format!("{:>3} {} {}", i + 1, if d.dirty { "*" } else { " " }, name),
                74,
            );
            c.text(
                12,
                7 + row,
                &format!("{text:74}"),
                if selected { WHITE } else { BLACK },
                if selected { BLUE } else { GRAY },
            );
        }
        c.text(12, 24, "[ Esc: Cancel ]", BLACK, GRAY);
    }
    pub(super) fn select_browser_entry(&mut self) {
        if let Some(b) = &self.browser {
            if let Some(e) = b.entries.get(b.selected) {
                if !e.dir {
                    self.entry = e.label.clone();
                }
            }
        }
    }
    pub(super) fn browser_up(&mut self) {
        if let Some(b) = self.browser.as_mut() {
            if let Some(p) = b.cwd.parent() {
                b.load(p.to_path_buf());
            }
            b.list_focus = true;
        }
    }
    pub(super) fn browser_filter(&mut self) {
        if let Some(b) = self.browser.as_mut() {
            b.all_files = !b.all_files;
            b.load(b.cwd.clone());
        }
    }
    pub(super) fn browser_key(&mut self, key: Key, ctrl: bool) {
        if key == Key::Escape {
            self.cancel_dialog();
            return;
        }
        if key == Key::Enter {
            self.accept_browser();
            return;
        }
        if ctrl && key == Key::A {
            self.browser_filter();
            return;
        }
        if ctrl && key == Key::U {
            self.browser_up();
            return;
        }
        let b = self.browser.as_mut().unwrap();
        match key {
            Key::Tab => b.list_focus = !b.list_focus,
            Key::Up => {
                b.list_focus = true;
                b.selected = b.selected.saturating_sub(1);
            }
            Key::Down => {
                b.list_focus = true;
                b.selected = (b.selected + 1).min(b.entries.len().saturating_sub(1));
            }
            Key::PageUp => {
                b.list_focus = true;
                b.selected = b.selected.saturating_sub(LIST_ROWS);
            }
            Key::PageDown => {
                b.list_focus = true;
                b.selected = (b.selected + LIST_ROWS).min(b.entries.len().saturating_sub(1));
            }
            Key::Home if b.list_focus => b.selected = 0,
            Key::End if b.list_focus => b.selected = b.entries.len().saturating_sub(1),
            Key::Backspace if b.list_focus => {
                self.browser_up();
                return;
            }
            Key::Backspace => {
                self.entry.pop();
            }
            Key::Delete if !b.list_focus => self.entry.clear(),
            _ => (),
        }
        b.reveal();
        if b.list_focus {
            self.select_browser_entry();
        }
    }
    pub(super) fn browser_click(&mut self, x: usize, y: usize) {
        if (10..90).contains(&x) && (7..7 + LIST_ROWS).contains(&y) {
            let b = self.browser.as_mut().unwrap();
            let i = b.top + y - 7;
            if i < b.entries.len() {
                let double = b
                    .last_click
                    .map(|(prev, t)| prev == i && t.elapsed() < Duration::from_millis(450))
                    .unwrap_or(false);
                b.selected = i;
                b.list_focus = true;
                b.last_click = Some((i, Instant::now()));
                self.select_browser_entry();
                if double {
                    self.accept_browser();
                }
            }
        } else if y == 24 && (16..91).contains(&x) {
            self.browser.as_mut().unwrap().list_focus = false;
        } else if y == 29 {
            if (10..26).contains(&x) {
                self.accept_browser();
            } else if (28..42).contains(&x) {
                self.browser_up();
            } else if (44..63).contains(&x) {
                self.browser_filter();
            } else if (67..86).contains(&x) {
                self.cancel_dialog();
            }
        }
    }
    pub(super) fn accept_browser(&mut self) {
        let Some(b) = self.browser.as_mut() else {
            return;
        };
        let mut path = b.path(&self.entry);
        if b.list_focus {
            if let Some(e) = b.entries.get(b.selected) {
                path = e.path.clone();
            } else {
                b.error = "No entry selected. Tab selects the filename field.".into();
                return;
            }
        }
        if path.is_dir() {
            b.load(path);
            b.list_focus = true;
            self.entry.clear();
            return;
        }
        if self.entry.is_empty() && !b.list_focus {
            b.error = "Enter a file name.".into();
            return;
        }
        match self.dialog.clone() {
            Some(Dialog::Open) => {
                // Keep the browser and its directory when opening fails.
                match std::fs::read_to_string(&path) {
                    Ok(_) => {
                        let count = self.docs.len();
                        self.open(path);
                        if self.docs.len() != count || self.status == "File opened" {
                            self.dialog = None;
                            self.browser = None;
                        }
                    }
                    Err(e) => b.error = format!("Open failed: {e}"),
                }
            }
            Some(Dialog::Save(after)) => {
                if path.extension().is_none() {
                    path.set_extension("pas");
                }
                if let Some(i) = self.docs.iter().position(|d| {
                    d.path
                        .as_ref()
                        .map(|p| *p == std::fs::canonicalize(&path).unwrap_or(path.clone()))
                        .unwrap_or(false)
                }) {
                    if i != self.active {
                        b.error =
                            "This file is open in another editor. Close that editor first.".into();
                        return;
                    }
                }
                if path.exists() {
                    self.browser = None;
                    self.dialog = Some(Dialog::Overwrite(path, after));
                    self.entry.clear();
                } else {
                    self.save_to(path, after);
                    if self.dialog.is_some() {
                        if let Some(b) = self.browser.as_mut() {
                            b.error = self.status.clone();
                        }
                    }
                }
            }
            _ => (),
        }
    }
    pub(super) fn render_browser(&self, c: &mut Canvas) {
        let b = self.browser.as_ref().unwrap();
        let save = matches!(self.dialog, Some(Dialog::Save(_)));
        c.box_at(
            6,
            2,
            88,
            30,
            if save {
                "Save Pascal file"
            } else {
                "Open Pascal file"
            },
            BLACK,
            GRAY,
        );
        c.text(9, 4, &tail(&b.cwd.display().to_string(), 81), BLUE, GRAY);
        c.text(
            9,
            5,
            if b.all_files {
                "Folder / file                         Filter: all files"
            } else {
                "Folder / file                         Filter: *.pas *.pp *.inc"
            },
            BLACK,
            GRAY,
        );
        c.rect(9 * CW, 7 * CH, 82 * CW, LIST_ROWS * CH, BLUE);
        for (row, e) in b.entries.iter().skip(b.top).take(LIST_ROWS).enumerate() {
            let selected = b.top + row == b.selected;
            let text = clipped(
                &format!("{} {}", if e.dir { "[DIR]" } else { "     " }, e.label),
                78,
            );
            c.text(
                10,
                7 + row,
                &format!("{text:78}"),
                if selected { BLACK } else { WHITE },
                if selected { CYAN } else { BLUE },
            );
        }
        c.text(
            10,
            22,
            &format!(
                "{} entries. Tab: list/name. Enter: open folder or accept.",
                b.entries.len()
            ),
            BLACK,
            GRAY,
        );
        c.text(9, 24, "Name:", BLACK, GRAY);
        c.rect(16 * CW, 24 * CH, 75 * CW, CH, BLUE);
        let name = tail(&self.entry, 73);
        c.text(17, 24, &name, YELLOW, BLUE);
        if !b.list_focus {
            c.rect((17 + name.chars().count()) * CW, 25 * CH - 2, CW, 2, WHITE);
        }
        c.text(9, 26, &clipped(&b.error, 81), RED, GRAY);
        c.text(
            9,
            27,
            "Delete: clear name. Ctrl+U: parent folder. Ctrl+A: filter.",
            BLACK,
            GRAY,
        );
        c.text(
            10,
            29,
            if save { "[ Save ]" } else { "[ Open ]" },
            BLUE,
            GRAY,
        );
        c.text(28, 29, "[ Up ]", BLUE, GRAY);
        c.text(44, 29, "[ All/Pascal ]", BLUE, GRAY);
        c.text(67, 29, "[ Cancel ]", BLUE, GRAY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "otp-window-tests-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::SeqCst)
            ));
            std::fs::create_dir_all(p.join("folder with spaces")).unwrap();
            std::fs::write(p.join("HELLO.PAS"), "begin end.").unwrap();
            std::fs::write(p.join("readme.txt"), "existing").unwrap();
            Self(std::fs::canonicalize(p).unwrap())
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn press(a: &mut App, k: Key) {
        a.keys(vec![k], false, false, false);
    }
    fn click(a: &mut App, x: usize, y: usize) {
        a.mouse_down = false;
        a.pointer(x, y, true, false);
        a.pointer(x, y, false, false);
    }
    #[test]
    fn tile_cascade_zoom_and_large_document_set() {
        let mut a = App::new();
        for _ in 0..16 {
            a.action("New");
        }
        a.tile();
        let active = a.active;
        let r = a.doc().rect;
        for d in &a.docs {
            assert_eq!(d.rect, d.rect.bounded());
        }
        a.keys(vec![Key::F5], false, false, false);
        assert_eq!(a.doc().rect, Rect::desktop());
        press(&mut a, Key::F5);
        assert_eq!(a.doc().rect, r);
        a.cascade();
        assert_eq!(a.zorder.last(), Some(&active));
        press(&mut a, Key::F6);
        assert_eq!(a.active, 0);
        a.keys(vec![Key::F6], false, false, true);
        assert_eq!(a.active, 16);
        // Render all layouts at their smallest supported size, including cursor clipping.
        a.doc_mut().rect = Rect {
            x: 76,
            y: 17,
            w: 24,
            h: 6,
        };
        a.render(&mut Canvas::new());
    }
    #[test]
    fn title_click_focus_drag_resize_and_close() {
        let mut a = App::new();
        a.action("New");
        a.tile();
        click(&mut a, 15, 1);
        assert_eq!(a.active, 0);
        a.mouse_down = false;
        a.pointer(15, 1, true, false);
        a.mouse_down = true;
        a.pointer(35, 4, true, false);
        a.pointer(35, 4, false, false);
        assert_eq!(a.doc().rect.x, 20);
        assert_eq!(a.doc().rect.y, 1); // Full-height tile is bounded at y=1.
        let r = a.doc().rect;
        a.mouse_down = false;
        a.pointer(r.x + r.w - 1, r.y + r.h - 1, true, false);
        a.mouse_down = true;
        a.pointer(99, 22, true, false);
        a.pointer(99, 22, false, false);
        assert_eq!(a.doc().rect.x + a.doc().rect.w, 100);
        click(&mut a, 21, 1);
        assert_eq!(a.docs.len(), 1);
        assert!(a.doc().dirty);
        a.action("Close     Alt+F3");
        assert!(matches!(a.dialog, Some(Dialog::Close)));
        press(&mut a, Key::Escape);
        assert_eq!(a.docs.len(), 1);
        a.action("Close     Alt+F3");
        a.entry = "DISCARD".into();
        a.accept();
        assert_eq!(a.docs.len(), 1);
        assert!(!a.doc().dirty);
        assert_eq!(a.doc().text(), "");
    }
    #[test]
    fn scroll_offsets_survive_render_and_click_maps_scrolled_text() {
        let mut a = App::new();
        a.docs[0] = Document::new(
            &(0..100)
                .map(|n| format!("{n}:{}", "x".repeat(180)))
                .collect::<Vec<_>>()
                .join("\n"),
            None,
        );
        a.render(&mut Canvas::new());
        a.wheel(20, 5, 0.0, -5.0, false);
        a.render(&mut Canvas::new());
        assert_eq!(a.doc().top, 15);
        a.wheel(20, 5, 0.0, -4.0, true);
        a.render(&mut Canvas::new());
        assert_eq!(a.doc().left, 12);
        click(&mut a, 10, 3);
        assert_eq!((a.doc().row, a.doc().col), (16, 15));
        // Keyboard movement reveals the caret again after a manual scroll.
        a.wheel(20, 5, 0.0, -10.0, false);
        press(&mut a, Key::Left);
        a.render(&mut Canvas::new());
        assert!(a.doc().top <= a.doc().row);
        // Scrollbar drag to bottom never forces the cursor into view.
        let r = a.doc().rect;
        a.mouse_down = false;
        a.pointer(99, 10, true, false);
        a.mouse_down = true;
        a.pointer(99, 21, true, false);
        a.pointer(99, 21, false, false);
        a.render(&mut Canvas::new());
        assert_eq!(a.doc().top, 100 - r.rows());
    }
    #[test]
    fn file_browser_filter_parent_errors_and_open_keyboard() {
        let f = Fixture::new();
        let mut a = App::new();
        a.dialog(Dialog::Open, f.0.display().to_string());
        let b = a.browser.as_ref().unwrap();
        assert!(b.entries.iter().any(|e| e.label == "HELLO.PAS"));
        assert!(!b.entries.iter().any(|e| e.label == "readme.txt"));
        a.browser_key(Key::A, true);
        assert!(a
            .browser
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .any(|e| e.label == "readme.txt"));
        a.entry = "folder with spaces".into();
        a.accept();
        assert!(a
            .browser
            .as_ref()
            .unwrap()
            .cwd
            .ends_with("folder with spaces"));
        a.browser_key(Key::U, true);
        assert_eq!(a.browser.as_ref().unwrap().cwd, f.0);
        a.browser.as_mut().unwrap().list_focus = false;
        a.entry = "missing.pas".into();
        a.accept();
        assert!(a.browser.as_ref().unwrap().error.starts_with("Open failed"));
        a.entry = "HELLO.PAS".into();
        press(&mut a, Key::Enter);
        assert!(a.dialog.is_none());
        assert_eq!(a.doc().text(), "begin end.");
        let count = a.docs.len();
        a.open(f.0.join("HELLO.PAS"));
        assert_eq!(a.docs.len(), count);
    }
    #[test]
    fn browser_mouse_folder_and_filename_field() {
        let f = Fixture::new();
        let mut a = App::new();
        a.dialog(Dialog::Open, f.0.display().to_string());
        let i = a
            .browser
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .position(|e| e.label == "folder with spaces")
            .unwrap();
        click(&mut a, 20, 7 + i);
        click(&mut a, 20, 7 + i);
        assert!(a
            .browser
            .as_ref()
            .unwrap()
            .cwd
            .ends_with("folder with spaces"));
        click(&mut a, 30, 29);
        assert_eq!(a.browser.as_ref().unwrap().cwd, f.0);
        click(&mut a, 20, 24);
        a.entry = "HELLO.PAS".into();
        click(&mut a, 12, 29);
        assert!(a.dialog.is_none());
        assert_eq!(a.doc().name(), "HELLO.PAS");
    }
    #[test]
    fn save_as_confirmation_cancel_and_close_save() {
        let f = Fixture::new();
        let mut a = App::new();
        a.doc_mut().insert("changed");
        a.dialog(
            Dialog::Save(None),
            f.0.join("readme.txt").display().to_string(),
        );
        a.accept();
        assert!(matches!(a.dialog, Some(Dialog::Overwrite(_, _))));
        assert_eq!(
            std::fs::read_to_string(f.0.join("readme.txt")).unwrap(),
            "existing"
        );
        press(&mut a, Key::Escape);
        assert!(a.doc().dirty);
        a.action("Close     Alt+F3");
        a.entry = "SAVE".into();
        a.accept();
        assert!(a.close_after_save);
        // Cancel a pending close/save leaves the document untouched.
        press(&mut a, Key::Escape);
        assert!(!a.close_after_save);
        assert!(a.doc().dirty);
        a.action("Close     Alt+F3");
        a.entry = "SAVE".into();
        a.accept();
        a.entry = f.0.join("new program").display().to_string();
        a.accept();
        assert!(f.0.join("new program.pas").exists());
        assert_eq!(a.doc().text(), "");
        a.doc_mut().insert("replacement");
        a.dialog(
            Dialog::Save(None),
            f.0.join("readme.txt").display().to_string(),
        );
        a.accept();
        a.entry = "YES".into();
        a.accept();
        assert_eq!(
            std::fs::read_to_string(f.0.join("readme.txt")).unwrap(),
            "replacement"
        );
        assert!(!a.doc().dirty);
    }
    #[test]
    fn save_failure_and_other_open_file_do_not_lose_changes() {
        let f = Fixture::new();
        let mut a = App::new();
        a.open(f.0.join("HELLO.PAS"));
        a.action("New");
        a.dialog(
            Dialog::Save(None),
            f.0.join("HELLO.PAS").display().to_string(),
        );
        a.accept();
        assert!(a.browser.as_ref().unwrap().error.contains("another editor"));
        assert!(a.doc().dirty);
        a.entry = f.0.join("missing/sub/new.pas").display().to_string();
        a.accept();
        assert!(a.browser.as_ref().unwrap().error.contains("Save failed"));
        assert!(a.doc().dirty);
        assert_eq!(
            std::fs::read_to_string(f.0.join("HELLO.PAS")).unwrap(),
            "begin end."
        );
    }
    #[test]
    fn running_program_locks_document_switch_and_close() {
        let mut a = App::new();
        a.action("New");
        a.tile();
        a.running = true;
        press(&mut a, Key::F6);
        assert_eq!(a.active, 1);
        click(&mut a, 15, 1);
        assert_eq!(a.active, 1);
        a.action("Close     Alt+F3");
        assert_eq!(a.docs.len(), 2);
        assert!(a.dialog.is_none());
        a.action("Save as...");
        assert!(a.dialog.is_none());
        a.keys(vec![Key::F5], true, false, false);
        assert!(a.output_full);
        // Layout controls do not change the source identity during execution.
        a.action("Cascade");
        a.action("Zoom      F5");
        assert_eq!(a.active, 1);
    }
    #[test]
    fn window_list_keyboard_and_mouse_select_documents() {
        let mut a = App::new();
        for _ in 0..20 {
            a.action("New");
        }
        a.action("List...");
        press(&mut a, Key::Home);
        press(&mut a, Key::Down);
        press(&mut a, Key::Enter);
        assert_eq!(a.active, 1);
        a.action("List...");
        click(&mut a, 20, 7);
        assert_eq!(a.active, 0);
        a.action("List...");
        press(&mut a, Key::End);
        a.render(&mut Canvas::new());
        press(&mut a, Key::Enter);
        assert_eq!(a.active, 20);
    }
}
