use crate::util::color::{ThemeColor, rgba_to_f32, srgb_to_linear_rgba};

use super::TerminalView;

impl TerminalView {
    pub(super) const DEFAULT_CARET_COLOR: [f32; 4] = ThemeColor::WHITE_ALPHA;

    pub(super) fn shell_caret_visible(&self) -> bool {
        self.snapshot.cursor_visible && !self.ime_preedit.is_active() && !self.copy_mode.active
    }

    pub(crate) fn caret_rect(&mut self) -> (f32, f32, f32, f32) {
        if !self.shell_caret_visible() {
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
        let caret_y = Self::terminal_row_y(visible_row, self.config.line_height);

        (
            caret_x,
            caret_y,
            self.glyphs.cell_width,
            self.config.line_height,
        )
    }

    pub(crate) fn caret_color(&self, srgb_target: bool) -> [f32; 4] {
        match self.snapshot.cursor_color {
            Some(color) if srgb_target => srgb_to_linear_rgba(color),
            Some(color) => rgba_to_f32(color),
            None => Self::DEFAULT_CARET_COLOR,
        }
    }
}
