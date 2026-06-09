use crate::{terminal_view::glyph::GlyphonResources, util};

use super::TerminalView;

impl TerminalView {
    pub(crate) fn caret_rect(&mut self) -> (f32, f32, f32, f32) {
        let row_count: usize = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end): (usize, usize) = self.viewport.visible_range(row_count);

        if self.snapshot.cursor_row < start || self.snapshot.cursor_row >= end {
            return (Self::PADDING_X, self.terminal_origin_y(), 0.0, 0.0);
        }

        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system, &self.config)
                .max(1.0);

        let visible_row: usize = self.snapshot.cursor_row - start;
        let caret_x: f32 = Self::PADDING_X + self.snapshot.cursor_col as f32 * cell_width;
        let caret_y: f32 = if self.tab_bar.titles().len() <= 1 {
            Self::PADDING_Y + visible_row as f32 * self.config.line_height
        } else {
            Self::PADDING_Y + self.config.line_height + visible_row as f32 * self.config.line_height
        };

        (caret_x, caret_y, cell_width, self.config.line_height)
    }

    pub(crate) fn caret_color(&self) -> [f32; 4] {
        self.snapshot
            .cursor_color
            .map(util::color::rgba_to_f32)
            .unwrap_or(Self::DEFAULT_CARET_COLOR)
    }
}
