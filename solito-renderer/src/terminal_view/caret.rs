use crate::util::{self, color::ThemeColor};

use super::TerminalView;

impl TerminalView {
    pub(super) const DEFAULT_CARET_COLOR: [f32; 4] = ThemeColor::WHITE_ALPHA;

    pub(crate) fn caret_rect(&mut self) -> (f32, f32, f32, f32) {
        let row_count = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end) = self.viewport.visible_range(row_count);

        if self.snapshot.cursor_row < start || self.snapshot.cursor_row >= end {
            return (Self::PADDING_X, self.terminal_origin_y(), 0.0, 0.0);
        }

        let visible_row = self.snapshot.cursor_row - start;
        let caret_x = Self::PADDING_X + self.snapshot.cursor_col as f32 * self.glyphs.cell_width;
        let caret_y = if self.tab_bar.titles().len() <= 1 {
            Self::PADDING_Y + visible_row as f32 * self.config.line_height
        } else {
            Self::PADDING_Y + self.config.line_height + visible_row as f32 * self.config.line_height
        };

        (
            caret_x,
            caret_y,
            self.glyphs.cell_width,
            self.config.line_height,
        )
    }

    pub(crate) fn caret_color(&self) -> [f32; 4] {
        self.snapshot
            .cursor_color
            .map(util::color::rgba_to_f32)
            .unwrap_or(Self::DEFAULT_CARET_COLOR)
    }

    fn terminal_origin_y(&self) -> f32 {
        Self::terminal_origin_y_for(self.config.line_height)
    }

    fn terminal_origin_y_for(line_height: f32) -> f32 {
        Self::PADDING_Y + Self::tab_bar_height_for(line_height)
    }
}
