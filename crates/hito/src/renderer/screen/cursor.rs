pub(super) struct Cursor {
    col: usize,
    row: usize,
}

impl Cursor {
    pub(super) fn new() -> Self {
        Self {
            col: usize::default(),
            row: usize::default(),
        }
    }
    pub(super) fn reset_col(&mut self) {
        self.col = 0;
    }

    pub(super) fn forward_col(&mut self) {
        self.col += 1;
    }

    pub(super) fn line_feed(&mut self) {
        self.row += 1;
    }

    pub(super) fn col(&self) -> usize {
        self.col
    }

    pub(super) fn row(&self) -> usize {
        self.row
    }

    pub(super) fn move_to(&mut self, row: u16, col: u16) {
        // Terminal cursor positions are 1-based; the internal buffer indexes are 0-based.
        self.row = row.saturating_sub(1) as usize;
        self.col = col.saturating_sub(1) as usize;
    }
}
