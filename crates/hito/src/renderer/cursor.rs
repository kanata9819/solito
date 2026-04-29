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
}
