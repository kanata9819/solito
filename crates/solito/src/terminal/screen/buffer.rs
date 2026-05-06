use super::cursor::Cursor;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellStyle {
    pub faint: bool,
    pub fg_rgba: Option<[u8; 4]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenCell {
    pub ch: char,
    pub style: CellStyle,
    pub is_wide_continuation: bool,
}

impl ScreenCell {
    pub fn new(ch: char, style: CellStyle) -> Self {
        Self {
            ch,
            style,
            is_wide_continuation: false,
        }
    }

    pub fn blank(style: CellStyle) -> Self {
        Self {
            ch: ' ',
            style,
            is_wide_continuation: false,
        }
    }

    pub fn wide_continuation(style: CellStyle) -> Self {
        Self {
            ch: ' ',
            style,
            is_wide_continuation: true,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScreenSnapshot {
    pub lines: Vec<Vec<ScreenCell>>,
    #[allow(dead_code)]
    pub cursor_row: usize,
    #[allow(dead_code)]
    pub cursor_col: usize,
}

pub struct ScreenBuffer {
    cols: usize,
    rows: usize,
    pub lines: Vec<Vec<ScreenCell>>,
    pub cursor: Cursor,
    pub pending_wrap: bool,
    pub style: CellStyle,
}

impl ScreenBuffer {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
            lines: vec![Vec::new()],
            cursor: Cursor::new(),
            pending_wrap: false,
            style: CellStyle::default(),
        }
    }

    pub fn set_cols(&mut self, cols: usize) {
        self.cols = cols.max(1);
    }

    pub fn set_rows(&mut self, rows: usize) {
        self.rows = rows.max(1);
    }

    pub fn snapshot(&self) -> ScreenSnapshot {
        ScreenSnapshot {
            lines: self.lines.clone(),
            cursor_row: self.cursor.get_current_row(),
            cursor_col: self.cursor.get_current_col(),
        }
    }

    pub(crate) fn cols(&self) -> usize {
        self.cols
    }

    pub(crate) fn ensure_cursor_line(&mut self) {
        while self.lines.len() <= self.cursor.get_current_row() {
            self.lines.push(Vec::new());
        }
    }

    pub(crate) fn ensure_cursor_col(&mut self) {
        self.ensure_cursor_line();
        let line: &mut Vec<ScreenCell> = &mut self.lines[self.cursor.get_current_row()];

        while line.len() < self.cursor.get_current_col() {
            line.push(ScreenCell::blank(self.style));
        }
    }

    #[allow(dead_code)]
    pub fn cursor_position_1_based(&self) -> (usize, usize) {
        let viewport_top: usize = self.lines.len().saturating_sub(self.rows);
        let visible_row: usize = self.cursor.get_current_row().saturating_sub(viewport_top);

        (visible_row + 1, self.cursor.get_current_col() + 1)
    }

    pub fn get_viewport_top(&self) -> usize {
        self.lines.len().saturating_sub(self.rows)
    }
}
