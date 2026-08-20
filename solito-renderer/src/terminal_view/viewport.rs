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
        let end = row_count.saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(self.visible_rows);
        (start, end)
    }

    pub(super) fn clamp(&mut self, row_count: usize) {
        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset(row_count));
    }

    pub(super) fn reset(&mut self) {
        self.scroll_offset = 0;
        self.scroll_accumulator = 0.0;
    }

    pub(super) fn is_at_bottom(&self) -> bool {
        self.scroll_offset == 0
    }

    pub(super) fn scroll_to_start(&mut self, start: usize, row_count: usize) {
        let start = start.min(row_count.saturating_sub(self.visible_rows));
        let end = start.saturating_add(self.visible_rows).min(row_count);
        self.scroll_offset = row_count.saturating_sub(end);
        self.scroll_accumulator = 0.0;
        self.clamp(row_count);
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

    pub(super) fn scroll_to_include(&mut self, row: usize, row_count: usize) -> bool {
        self.clamp(row_count);
        let previous_range = self.visible_range(row_count);
        let (start, end) = previous_range;

        if row < start {
            let desired_end = row.saturating_add(self.visible_rows).min(row_count);
            self.scroll_offset = row_count.saturating_sub(desired_end);
        } else if row >= end {
            self.scroll_offset = row_count.saturating_sub(row.saturating_add(1));
        }

        self.scroll_accumulator = 0.0;
        self.clamp(row_count);

        self.visible_range(row_count) != previous_range
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

    #[test]
    fn scroll_to_include_reincludes_same_cursor_after_rows_grow() {
        let mut viewport = ViewportState::new(30, 10.0);

        assert!(!viewport.scroll_to_include(4, 5));
        assert_eq!(viewport.visible_range(5), (2, 5));

        assert!(viewport.scroll_to_include(4, 10));
        assert_eq!(viewport.visible_range(10), (4, 7));
    }

    #[test]
    fn scroll_to_include_reports_no_change_for_visible_cursor() {
        let mut viewport = ViewportState::new(30, 10.0);

        assert!(!viewport.scroll_to_include(8, 10));
        assert_eq!(viewport.visible_range(10), (7, 10));
    }
}
