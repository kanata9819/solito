pub struct Cursor {
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
}
