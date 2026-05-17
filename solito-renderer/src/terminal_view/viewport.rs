pub(super) struct ViewportState {
    line_height: f32,
    scroll_offset: usize,
    scroll_accumulator: f32,
    visible_rows: usize,
}

impl ViewportState {
    pub(super) fn new(height: u32, line_height: f32) -> Self {
        Self {
            line_height,
            scroll_offset: 0,
            scroll_accumulator: 0.0,
            visible_rows: Self::visible_rows(height, line_height),
        }
    }

    pub(super) fn resize(&mut self, height: u32, row_count: usize) {
        self.visible_rows = Self::visible_rows(height, self.line_height);
        self.clamp(row_count);
    }

    pub(super) fn visible_range(&self, row_count: usize) -> (usize, usize) {
        let end: usize = row_count.saturating_sub(self.scroll_offset);
        let start: usize = end.saturating_sub(self.visible_rows);
        (start, end)
    }

    pub(super) fn clamp(&mut self, row_count: usize) {
        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset(row_count));
    }

    pub(super) fn reset(&mut self) {
        self.scroll_offset = 0;
        self.scroll_accumulator = 0.0;
    }

    pub(super) fn scroll(&mut self, y: f32, row_count: usize) {
        self.scroll_accumulator += y;

        while self.scroll_accumulator >= 1.0 {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
            self.scroll_accumulator -= 1.0;
        }

        while self.scroll_accumulator <= -1.0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
            self.scroll_accumulator += 1.0;
        }

        self.clamp(row_count);
    }

    fn visible_rows(height: u32, line_height: f32) -> usize {
        ((height as f32 / line_height).floor() as usize).max(1)
    }

    fn max_scroll_offset(&self, row_count: usize) -> usize {
        row_count.saturating_sub(self.visible_rows)
    }
}
