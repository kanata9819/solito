use super::buffer::CellStyle;

#[derive(Clone, Default, Debug)]
pub(super) struct Cursor {
    cursor_col: usize,
    cursor_row: usize,
    saved_cursor: Option<SavedCursor>,
}

/// State saved by DECSC. The row is relative to the visible screen, like
/// xterm, so output that scrolls the screen does not move the restore target.
#[derive(Debug, Clone, Copy)]
pub(super) struct SavedCursor {
    pub(super) screen_row: usize,
    pub(super) col: usize,
    pub(super) style: CellStyle,
    pub(super) origin_mode: bool,
    pub(super) auto_wrap: bool,
    pub(super) pending_wrap: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CursorPosition {
    pub(super) row: usize,
    pub(super) col: usize,
}

impl Cursor {
    pub(super) fn discard_rows(&mut self, count: usize) {
        self.cursor_row = self.cursor_row.saturating_sub(count);
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

    pub(super) fn save(&mut self, saved: SavedCursor) {
        self.saved_cursor = Some(saved);
    }

    pub(super) fn saved(&self) -> Option<SavedCursor> {
        self.saved_cursor
    }
}
