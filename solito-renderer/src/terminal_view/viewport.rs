pub(super) struct ViewportState {
    line_height: f32,
    scroll_offset: f32,
    scroll_accumulator: f32,
    visible_rows: usize,
}

impl ViewportState {
    pub(super) fn new(height: u32, line_height: f32) -> Self {
        Self {
            line_height,
            scroll_offset: 0.0,
            scroll_accumulator: 0.0,
            visible_rows: Self::visible_rows(height, line_height),
        }
    }

    pub(super) fn resize(&mut self, height: u32, row_count: usize) {
        self.visible_rows = Self::visible_rows(height, self.line_height);
        self.clamp(row_count);
    }

    pub(super) fn visible_range(&self, row_count: usize) -> (usize, usize) {
        let end = row_count.saturating_sub(self.scroll_offset as usize);
        let start = end.saturating_sub(self.visible_rows);
        (start, end)
    }

    pub(super) fn clamp(&mut self, row_count: usize) {
        self.scroll_offset = self
            .scroll_offset
            .min(self.max_scroll_offset(row_count) as f32);
    }

    pub(super) fn reset(&mut self) {
        self.scroll_offset = 0.0;
        self.scroll_accumulator = 0.0;
    }

    pub(super) fn is_at_bottom(&self) -> bool {
        self.scroll_offset == 0.0
    }

    pub(super) fn scroll_to_start(&mut self, start: usize, row_count: usize) {
        let start = start.min(row_count.saturating_sub(self.visible_rows));
        let end = start.saturating_add(self.visible_rows).min(row_count);
        self.scroll_offset = row_count.saturating_sub(end as usize) as f32;
        self.scroll_accumulator = 0.0;
        self.clamp(row_count);
    }

    pub(super) fn scroll(&mut self, y: f32, row_count: usize) {
        // self.scroll_accumulator += y;

        if y < 0.0 {
            self.scroll_offset -= y;
        } else {
            self.scroll_offset += y;
        }

        self.clamp(row_count);
    }

    pub(super) fn scroll_to_include(&mut self, row: usize, row_count: usize) {
        self.clamp(row_count);
        let (start, end) = self.visible_range(row_count);

        if row < start {
            let desired_end = row.saturating_add(self.visible_rows).min(row_count);
            self.scroll_offset = row_count.saturating_sub(desired_end) as f32;
        } else if row >= end {
            self.scroll_offset = row_count.saturating_sub(row.saturating_add(1)) as f32;
        }

        self.scroll_accumulator = 0.0;
        self.clamp(row_count);
    }

    fn visible_rows(height: u32, line_height: f32) -> usize {
        ((height as f32 / line_height).floor() as usize).max(1)
    }

    fn max_scroll_offset(&self, row_count: usize) -> usize {
        row_count.saturating_sub(self.visible_rows)
    }
}

#[cfg(test)]
mod tests {
    use super::ViewportState;

    #[test]
    fn scroll_to_start_preserves_visible_top_when_rows_grow() {
        let mut viewport = ViewportState::new(30, 10.0);

        viewport.scroll(4.0, 10);
        let (start, _) = viewport.visible_range(10);

        viewport.scroll_to_start(start, 12);

        assert_eq!(viewport.visible_range(12), (3, 6));
    }

    #[test]
    fn reset_returns_to_bottom() {
        let mut viewport = ViewportState::new(30, 10.0);

        viewport.scroll(4.0, 10);
        viewport.reset();

        assert!(viewport.is_at_bottom());
        assert_eq!(viewport.visible_range(10), (7, 10));
    }
}
