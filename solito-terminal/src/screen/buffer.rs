use super::cursor::Cursor;
use crate::TerminalSize;

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
    pub cursor_visible: bool,
}

#[derive(Clone, Default, Debug)]
pub(super) struct ScreenBuffer {
    cols: usize,
    rows: usize,
    pub(super) lines: Vec<Vec<ScreenCell>>,
    pub(super) cursor: Cursor,
    pub(super) pending_wrap: bool,
    pub(super) style: CellStyle,
    pub(super) cursor_color: Option<[u8; 4]>,
    pub(super) scroll_region: (usize, usize),
    pub(super) scroll_region_active: bool,
    pub(super) origin_mode: bool,
    pub(super) insert_mode: bool,
    pub(super) auto_wrap: bool,
    pub(super) cursor_visible: bool,
    pub(super) tab_stops: Vec<bool>,
}

impl ScreenBuffer {
    pub(super) fn new(size: TerminalSize) -> Self {
        Self {
            cols: size.cols,
            rows: size.rows,
            lines: vec![Vec::new()],
            cursor: Cursor::default(),
            pending_wrap: false,
            style: CellStyle::default(),
            cursor_color: None,
            scroll_region: (0, size.rows.saturating_sub(1)),
            scroll_region_active: false,
            origin_mode: false,
            insert_mode: false,
            auto_wrap: true,
            cursor_visible: true,
            tab_stops: default_tab_stops(size.cols),
        }
    }

    pub(super) fn resize(&mut self, size: TerminalSize) {
        self.cols = size.cols;
        self.rows = size.rows;
        self.scroll_region.0 = self.scroll_region.0.min(size.rows.saturating_sub(1));
        self.scroll_region.1 = self
            .scroll_region
            .1
            .clamp(self.scroll_region.0, size.rows.saturating_sub(1));
        self.tab_stops.resize_with(size.cols, || false);
        for (col, stop) in self.tab_stops.iter_mut().enumerate() {
            if col % 8 == 0 && !*stop {
                *stop = true;
            }
        }
    }

    pub(super) fn snapshot(&self) -> ScreenSnapshot {
        ScreenSnapshot {
            lines: self.lines.clone(),
            cursor_row: self.cursor.get_current_row(),
            cursor_col: self.cursor.get_current_col(),
            cursor_color: self.cursor_color,
            cursor_visible: self.cursor_visible,
        }
    }

    pub(super) fn cols(&self) -> usize {
        self.cols
    }

    pub(super) fn rows(&self) -> usize {
        self.rows
    }

    pub(super) fn ensure_cursor_line(&mut self) {
        while self.lines.len() <= self.cursor.get_current_row() {
            self.lines.push(Vec::new());
        }
    }

    pub(super) fn ensure_cursor_col(&mut self) {
        self.ensure_cursor_line();
        let line = &mut self.lines[self.cursor.get_current_row()];

        while line.len() < self.cursor.get_current_col() {
            line.push(ScreenCell::blank(self.style));
        }
    }

    pub(super) fn get_viewport_top(&self) -> usize {
        self.lines.len().saturating_sub(self.rows)
    }

    pub(super) fn insert_cell(&mut self, row: usize, col: usize, cell: ScreenCell) {
        self.lines[row].insert(col, cell);
    }

    pub(super) fn replace_cell(&mut self, row: usize, col: usize, cell: ScreenCell) {
        self.lines[row][col] = cell;
    }

    pub(super) fn push_cell(&mut self, row: usize, cell: ScreenCell) {
        self.lines[row].push(cell);
    }

    pub(super) fn line_len(&self, row: usize) -> usize {
        self.lines[row].len()
    }

    pub(super) fn truncate_line(&mut self, row: usize, cells_to_keep: usize) {
        self.lines[row].truncate(cells_to_keep);
    }
}

fn default_tab_stops(cols: usize) -> Vec<bool> {
    (0..cols).map(|col| col % 8 == 0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn init_buffer() -> ScreenBuffer {
        let mut buffer = ScreenBuffer::new(terminal_size());
        for ch in 'A'..='Z' {
            buffer.lines[0].push(cell(ch));
        }
        buffer
    }

    fn terminal_size() -> TerminalSize {
        TerminalSize {
            cols: 120,
            rows: 80,
        }
    }

    fn cell(c: char) -> ScreenCell {
        ScreenCell::new(c, CellStyle::default())
    }

    #[test]
    fn insert_cell_to_buffer() {
        let mut buf = init_buffer();
        assert_eq!(buf.lines[0][0].ch, 'A');

        buf.insert_cell(0, 0, cell('a'));
        assert_eq!(buf.lines[0][0].ch, 'a');
        assert_eq!(buf.lines[0][1].ch, 'A');
        assert_eq!(buf.lines[0][buf.line_len(0) - 1].ch, 'Z');
        assert_eq!(buf.lines[0].len(), 27);
    }

    #[test]
    fn replace_cell_for_buffer() {
        let mut buf = init_buffer();
        assert_eq!(buf.lines[0][0].ch, 'A');
        assert_eq!(buf.lines[0].len(), 26);

        buf.replace_cell(0, 0, cell('a'));
        assert_eq!(buf.lines[0][0].ch, 'a');
        assert_eq!(buf.lines[0].len(), 26);
        assert_eq!(buf.lines[0][1].ch, 'B');
    }

    #[test]
    fn push_cell_to_buffer() {
        let mut buf = init_buffer();
        assert_eq!(buf.lines[0][0].ch, 'A');
        assert_eq!(buf.lines[0].len(), 26);
        assert_eq!(buf.lines[0][buf.line_len(0) - 1].ch, 'Z');

        buf.push_cell(0, cell('a'));
        assert_eq!(buf.lines[0][0].ch, 'A');
        assert_eq!(buf.lines[0].len(), 27);
        assert_eq!(buf.lines[0][buf.line_len(0) - 1].ch, 'a');
    }

    #[test]
    fn truncate_buffer() {
        let mut buf = init_buffer();
        assert_eq!(buf.line_len(0), 26);

        buf.truncate_line(0, 1);
        assert_eq!(buf.line_len(0), 1);
        assert_eq!(buf.lines[0][0].ch, 'A');
    }
}
