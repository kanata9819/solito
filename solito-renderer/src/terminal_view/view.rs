use glyphon::FontSystem;
use solito_terminal::ScreenLine;
use solito_terminal::{ScreenSnapshot, TerminalSize};

use crate::RendererConfig;

use super::{
    copy_mode::CopyModeSnapshot, glyph::GlyphonResources, tab_bar::TabBarSnapshot,
    text_damage::TextDamage, viewport::ViewportState,
};

pub(crate) struct TerminalView {
    pub(crate) glyphs: GlyphonResources,
    pub(super) config: RendererConfig,
    pub(super) snapshot: ScreenSnapshot,
    pub(super) tab_bar: TabBarSnapshot,
    pub(super) viewport: ViewportState,
    pub(super) copy_mode: CopyModeSnapshot,
    pub(super) text_damage: TextDamage,
}

impl TerminalView {
    pub(crate) const PADDING_X: f32 = 10.0;
    pub(crate) const PADDING_Y: f32 = 10.0;

    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain: wgpu::TextureFormat,
        physical_size: winit::dpi::PhysicalSize<u32>,
        config: RendererConfig,
    ) -> Self {
        let config = config.sanitized();
        let mut glyphs = GlyphonResources::new(device, queue, swapchain, &config);

        Self::set_text_buffer_size(
            &mut glyphs,
            physical_size.width,
            physical_size.height,
            &config,
        );

        Self {
            glyphs,
            viewport: ViewportState::new(
                Self::terminal_content_height(physical_size.height, config.line_height),
                config.line_height,
            ),
            config,
            snapshot: ScreenSnapshot::default(),
            tab_bar: TabBarSnapshot::default(),
            copy_mode: CopyModeSnapshot::default(),
            text_damage: TextDamage::All,
        }
    }

    pub(crate) fn estimate_terminal_size(
        width: u32,
        height: u32,
        config: &RendererConfig,
    ) -> TerminalSize {
        let config = config.clone().sanitized();
        let mut font_system = FontSystem::new();
        let cell_width = GlyphonResources::measure_font_width(&mut font_system, &config).max(1.0);
        let content_width = Self::terminal_content_width(width);
        let content_height = Self::terminal_content_height(height, config.line_height);

        TerminalSize::new(
            ((content_width as f32 / cell_width).floor() as usize).max(1),
            ((content_height as f32 / config.line_height).floor() as usize).max(1),
        )
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32, snapshot: ScreenSnapshot) {
        self.snapshot = snapshot;
        Self::set_text_buffer_size(&mut self.glyphs, width, height, &self.config);
        self.viewport.resize(
            Self::terminal_content_height(height, self.config.line_height),
            self.row_count(),
        );
        self.invalidate_all_text();
    }

    pub(crate) fn set_snapshot(&mut self, snapshot: ScreenSnapshot) {
        let previous_range = self.viewport.visible_range(self.row_count());
        let keep_start = if self.viewport.is_at_bottom() {
            None
        } else {
            Some(self.viewport.visible_range(self.row_count()).0)
        };
        let removed = snapshot
            .history_start
            .saturating_sub(self.snapshot.history_start);
        let keep_start = keep_start.map(|start| start.saturating_sub(removed));
        let damage = TextDamage::between(&self.snapshot, &snapshot);

        self.snapshot = snapshot;

        if let Some(start) = keep_start {
            self.viewport.scroll_to_start(start, self.row_count());
        } else {
            self.viewport.clamp(self.row_count());
        }

        if removed > 0 || previous_range != self.viewport.visible_range(self.row_count()) {
            self.invalidate_all_text();
        } else {
            self.text_damage.merge(damage);
        }
    }

    pub(crate) fn set_snapshot_at_bottom(&mut self, snapshot: ScreenSnapshot) {
        self.snapshot = snapshot;
        self.viewport.reset();
        self.invalidate_all_text();
    }

    pub(crate) fn scroll(&mut self, _x: f32, y: f32) {
        let previous_range = self.viewport.visible_range(self.row_count());
        self.viewport.scroll(y, self.row_count());
        if previous_range != self.viewport.visible_range(self.row_count()) {
            self.invalidate_all_text();
        }
    }

    pub(crate) fn visible_cols(&self, width: u32) -> usize {
        let content_width = Self::terminal_content_width(width);

        ((content_width as f32 / self.glyphs.cell_width).floor() as usize).max(1)
    }

    pub(crate) fn cell_at(&self, x: f64, y: f64, size: TerminalSize) -> Option<(usize, usize)> {
        let top = Self::terminal_row_y(0, self.config.line_height, self.has_tab_bar());
        Self::cell_at_position(
            x,
            y,
            top,
            self.glyphs.cell_width,
            self.config.line_height,
            size,
        )
    }

    fn cell_at_position(
        x: f64,
        y: f64,
        top: f32,
        width: f32,
        height: f32,
        size: TerminalSize,
    ) -> Option<(usize, usize)> {
        let x = x - f64::from(Self::PADDING_X);
        let y = y - f64::from(top);
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return None;
        }
        let col = (x / f64::from(width)) as usize;
        let row = (y / f64::from(height)) as usize;
        (col < size.cols && row < size.rows).then_some((col, row))
    }

    pub(crate) fn visible_rows(&self, height: u32) -> usize {
        let content_height = Self::terminal_content_height(height, self.config.line_height);
        ((content_height as f32 / self.config.line_height).floor() as usize).max(1)
    }

    pub(super) fn terminal_row_y(visible_row: usize, line_height: f32, has_tab_bar: bool) -> f32 {
        if has_tab_bar {
            Self::PADDING_Y + line_height + visible_row as f32 * line_height
        } else {
            Self::PADDING_Y + visible_row as f32 * line_height
        }
    }

    pub(super) fn display_col_count(lines: &[ScreenLine], row: usize) -> usize {
        lines.get(row).map_or(1, |line| line.len().max(1))
    }

    pub(super) fn row_count(&self) -> usize {
        self.snapshot.lines.len().max(1)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mouse_cells_exclude_padding_and_tab_bar() {
        let cell = |x, y| {
            super::TerminalView::cell_at_position(
                x,
                y,
                40.0,
                10.0,
                30.0,
                solito_terminal::TerminalSize::new(80, 24),
            )
        };
        assert_eq!(cell(10.0, 40.0), Some((0, 0)));
        assert_eq!(cell(29.9, 70.0), Some((1, 1)));
        assert_eq!(cell(9.0, 40.0), None);
        assert_eq!(cell(10.0, 39.0), None);
        assert_eq!(cell(810.0, 40.0), None);
        assert_eq!(cell(10.0, 760.0), None);
    }
    use super::TerminalView;
    use crate::util::color::ThemeColor;
    use crate::{RendererConfig, util};

    #[test]
    fn rgba_to_f32_normalizes_color_channels() {
        assert_eq!(
            util::color::rgba_to_f32([0, 128, 255, 255]),
            [0.0, 128.0 / 255.0, 1.0, 1.0]
        );
    }

    #[test]
    fn caret_color_defaults_to_white() {
        assert_eq!(TerminalView::DEFAULT_CARET_COLOR, ThemeColor::WHITE_ALPHA);
    }

    #[test]
    fn visible_rows_reserve_one_row_for_tab_bar() {
        assert_eq!(
            TerminalView::terminal_content_height(90, RendererConfig::DEFAULT_LINE_HEIGHT),
            60
        );
    }

    #[test]
    fn terminal_content_width_reserves_horizontal_padding() {
        assert_eq!(TerminalView::terminal_content_width(100), 80);
        assert_eq!(TerminalView::terminal_content_width(10), 1);
    }

    #[test]
    fn terminal_row_y_accounts_for_the_tab_bar() {
        assert_eq!(TerminalView::terminal_row_y(2, 30.0, false), 70.0);
        assert_eq!(TerminalView::terminal_row_y(2, 30.0, true), 100.0);
    }
}
