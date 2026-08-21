use crate::util::{self, color::ThemeColor};

use super::TerminalView;

impl TerminalView {
    pub(super) const DEFAULT_CARET_COLOR: [f32; 4] = ThemeColor::WHITE_ALPHA;

    pub(crate) fn caret_rect(&mut self) -> (f32, f32, f32, f32) {
        if !self.snapshot.cursor_visible {
            return (Self::PADDING_X, Self::PADDING_Y, 0.0, 0.0);
        }

        let row_count = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end) = self.viewport.visible_range(row_count);

        if self.snapshot.cursor_row < start || self.snapshot.cursor_row >= end {
            return (
                Self::PADDING_X,
                Self::PADDING_Y + self.config.line_height,
                0.0,
                0.0,
            );
        }

        let visible_row = self.snapshot.cursor_row - start;
        let caret_x = Self::PADDING_X + self.snapshot.cursor_col as f32 * self.glyphs.cell_width;
        let caret_y =
            Self::terminal_row_y(visible_row, self.config.line_height, self.has_tab_bar());

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
}
