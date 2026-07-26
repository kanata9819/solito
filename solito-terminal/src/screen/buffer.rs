use super::cursor::Cursor;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CellStyle {
    pub(super) faint: bool,
    pub(super) fg_rgba: Option<[u8; 4]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenCell {
    pub ch: char,
    pub(super) style: CellStyle,
    pub is_wide_continuation: bool,
}

impl ScreenCell {
    pub(super) fn new(ch: char, style: CellStyle) -> Self {
        Self {
            ch,
            style,
            is_wide_continuation: false,
        }
    }

    pub(super) fn blank(style: CellStyle) -> Self {
        Self {
            ch: ' ',
            style,
            is_wide_continuation: false,
        }
    }

    pub(super) fn wide_continuation(style: CellStyle) -> Self {
        Self {
            is_wide_continuation: true,
            ..Self::blank(style)
        }
    }

    pub fn foreground_rgba(&self) -> Option<[u8; 4]> {
        self.style.fg_rgba
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScreenSnapshot {
    pub lines: Vec<Vec<ScreenCell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub cursor_color: Option<[u8; 4]>,
}

pub(super) struct ScreenBuffer {
    cols: usize,
    rows: usize,
    pub(super) lines: Vec<Vec<ScreenCell>>,
    pub(super) cursor: Cursor,
    pub(super) pending_wrap: bool,
    pub(super) style: CellStyle,
    pub(super) cursor_color: Option<[u8; 4]>,
}

impl ScreenBuffer {
    pub(super) fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
            lines: vec![Vec::new()],
            cursor: Cursor::default(),
            pending_wrap: false,
            style: CellStyle::default(),
            cursor_color: None,
        }
    }

    pub(super) fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols.max(1);
        self.rows = rows.max(1);
    }

    pub(super) fn snapshot(&self) -> ScreenSnapshot {
        ScreenSnapshot {
            lines: self.lines.clone(),
            cursor_row: self.cursor.get_current_row(),
            cursor_col: self.cursor.get_current_col(),
            cursor_color: self.cursor_color,
        }
    }

    pub(super) fn cols(&self) -> usize {
        self.cols
    }

    pub(super) fn ensure_cursor_line(&mut self) {
        while self.lines.len() <= self.cursor.get_current_row() {
            self.lines.push(Vec::new());
        }
    }

    pub(super) fn ensure_cursor_col(&mut self) {
        self.ensure_cursor_line();
        let line: &mut Vec<ScreenCell> = &mut self.lines[self.cursor.get_current_row()];

        while line.len() < self.cursor.get_current_col() {
            line.push(ScreenCell::blank(self.style));
        }
    }

    pub(super) fn get_viewport_top(&self) -> usize {
        self.lines.len().saturating_sub(self.rows)
    }
}
