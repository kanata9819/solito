pub(super) struct Cursor {
    cursor_col: usize,
    cursor_row: usize,
    saved_cursor: Option<CursorPosition>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CursorPosition {
    pub(super) row: usize,
    pub(super) col: usize,
}

impl Cursor {
    pub(super) fn new() -> Self {
        Self {
            cursor_col: 0,
            cursor_row: 0,
            saved_cursor: None,
        }
    }

    pub(super) fn get_current_row(&self) -> usize {
        self.cursor_row
    }

    pub(super) fn get_current_col(&self) -> usize {
        self.cursor_col
    }

    pub(super) fn move_left(&mut self) {
        self.cursor_col = self.cursor_col.saturating_sub(1);
    }

    pub(super) fn move_down(&mut self) {
        self.cursor_row += 1;
    }

    pub(super) fn reset_col(&mut self) {
        self.cursor_col = 0;
    }

    pub(super) fn reset_row(&mut self) {
        self.cursor_row = 0;
    }

    pub(super) fn move_to(&mut self, position: CursorPosition) {
        self.cursor_row = position.row;
        self.cursor_col = position.col;
    }

    pub(super) fn move_to_col(&mut self, col: usize) {
        self.cursor_col = col;
    }

    pub(super) fn move_to_row(&mut self, row: usize) {
        self.cursor_row = row;
    }

    pub(super) fn save_cursor_position(&mut self, position: CursorPosition) {
        self.saved_cursor = Some(CursorPosition {
            row: position.row,
            col: position.col,
        })
    }

    pub(super) fn get_saved_cursor_position(&self) -> Option<CursorPosition> {
        self.saved_cursor
    }
}
