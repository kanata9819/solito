use super::{cursor::Cursor, viewport::ViewportState};

pub(super) struct ScreenBuffer {
    cursor: Cursor,
    lines: Vec<Vec<char>>,
}

impl ScreenBuffer {
    pub(super) fn new() -> Self {
        Self {
            cursor: Cursor::new(),
            lines: vec![Vec::new()],
        }
    }

    pub(super) fn cursor_row(&self) -> usize {
        self.cursor.row()
    }

    pub(super) fn cursor_col(&self) -> usize {
        self.cursor.col()
    }

    pub(super) fn row_count(&self) -> usize {
        self.lines.len()
    }

    pub(super) fn visible_text(&self, viewport: &mut ViewportState) -> String {
        viewport.clamp(self.row_count());
        let (start, end) = viewport.visible_range(self.row_count());

        self.lines[start..end]
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub(super) fn push_char(&mut self, c: char) {
        self.ensure_line();
        self.set_char_at_cursor(c);
        self.cursor.forward_col();
    }

    pub(super) fn reset_col(&mut self) {
        self.cursor.reset_col();
    }

    pub(super) fn line_feed(&mut self) {
        self.cursor.line_feed();
        self.ensure_row();
    }

    pub(super) fn clear_line(&mut self) {
        self.ensure_row();
        self.ensure_col();
        self.lines[self.cursor.row()].truncate(self.cursor.col());
    }

    pub(super) fn move_cursor_to(&mut self, row: u16, col: u16) {
        self.cursor.move_to(row, col);
    }

    fn set_char_at_cursor(&mut self, c: char) {
        if let Some(line) = self.lines.get_mut(self.cursor.row())
            && let Some(cell) = line.get_mut(self.cursor.col())
        {
            *cell = c;
        }
    }

    fn ensure_line(&mut self) {
        self.ensure_row();
        self.ensure_col();
    }

    fn ensure_row(&mut self) {
        while self.lines.len() <= self.cursor.row() {
            self.lines.push(Vec::new());
        }
    }

    fn ensure_col(&mut self) {
        if let Some(line) = self.lines.get_mut(self.cursor.row()) {
            while line.len() <= self.cursor.col() {
                line.push(' ');
            }
        }
    }
}
