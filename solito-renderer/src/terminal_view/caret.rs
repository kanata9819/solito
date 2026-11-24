use crate::{
    pipeline::rect::RectSpec,
    util::color::{ThemeColor, target_rgba, with_alpha},
};

use super::TerminalView;

impl TerminalView {
    pub(super) const DEFAULT_CARET_COLOR: [u8; 4] = ThemeColor::ACCENT;
    const CARET_GLOW: f32 = 8.0;

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
        target_rgba(
            self.snapshot
                .cursor_color
                .unwrap_or(Self::DEFAULT_CARET_COLOR),
            srgb_target,
        )
    }

    /// The block caret plus a soft halo in its own color.
    pub(crate) fn caret_rects(&mut self, srgb_target: bool) -> Vec<RectSpec> {
        let (x, y, w, h) = self.caret_rect();
        if w <= 0.0 || h <= 0.0 {
            return Vec::new();
        }
        let color = self.caret_color(srgb_target);
        let glow = Self::CARET_GLOW;
        vec![
            RectSpec::rounded(
                x - glow,
                y - glow,
                w + glow * 2.0,
                h + glow * 2.0,
                with_alpha(color, 0.28),
                glow + 2.0,
            )
            .soft(glow),
            RectSpec::rounded(x, y, w, h, color, 2.0),
        ]
    }
}
