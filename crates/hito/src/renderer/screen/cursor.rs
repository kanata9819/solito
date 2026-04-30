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

    pub(super) fn row(&self) -> usize {
        self.row
    }

    pub(super) fn move_to(&mut self, row: u16, col: u16) {
        self.row = row as usize;
        self.col = col as usize;
    }
}
