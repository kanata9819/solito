pub struct Cursor {
    col: usize,
    row: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            col: usize::default(),
            row: usize::default(),
        }
    }

    pub fn forward_col(&mut self) {
        self.col += 1;
    }

    pub fn backward_col(&mut self) {
        self.col = self.col.saturating_sub(1);
    }

    pub fn reset_col(&mut self) {
        self.col = 0;
    }

    pub fn forward_row(&mut self) {
        self.row += 1;
    }

    pub fn backward_row(&mut self) {
        self.row = self.row.saturating_sub(1);
    }

    pub fn reset_row(&mut self) {
        self.row = 0;
    }
}
