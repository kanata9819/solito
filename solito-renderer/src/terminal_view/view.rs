use glyphon::FontSystem;
use solito_terminal::{ScreenCell, ScreenSnapshot};

use crate::RendererConfig;

use super::{
    copy_mode::CopyModeSnapshot, glyph::GlyphonResources, tab_bar::TabBarSnapshot,
    viewport::ViewportState,
};

pub(crate) struct TerminalView {
    pub(crate) glyphs: GlyphonResources,
    pub(super) config: RendererConfig,
    pub(super) snapshot: ScreenSnapshot,
    pub(super) tab_bar: TabBarSnapshot,
    pub(super) viewport: ViewportState,
    pub(super) copy_mode: CopyModeSnapshot,
}

impl TerminalView {
    pub(crate) const PADDING_X: f32 = 10.0;
    pub(crate) const PADDING_Y: f32 = 10.0;

    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain: wgpu::TextureFormat,
        physical_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
        config: RendererConfig,
    ) -> Self {
        let config: RendererConfig = config.sanitized();
        let mut glyphs: GlyphonResources = GlyphonResources::new(
            device,
            queue,
            swapchain,
            physical_size,
            scale_factor,
            &config,
        );

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
        }
    }

    pub(crate) fn estimate_terminal_size(
        width: u32,
        height: u32,
        config: &RendererConfig,
    ) -> (usize, usize) {
        let config: RendererConfig = config.clone().sanitized();
        let mut font_system: FontSystem = FontSystem::new();
        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut font_system, &config).max(1.0);
        let content_width: u32 = Self::terminal_content_width(width);
        let content_height: u32 = Self::terminal_content_height(height, config.line_height);

        (
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
        self.set_text_to_buffer();
    }

    pub(crate) fn set_snapshot(&mut self, snapshot: ScreenSnapshot) {
        let keep_start: Option<usize> = if self.viewport.is_at_bottom() {
            None
        } else {
            Some(self.viewport.visible_range(self.row_count()).0)
        };

        self.snapshot = snapshot;

        if let Some(start) = keep_start {
            self.viewport.scroll_to_start(start, self.row_count());
        } else {
            self.viewport.clamp(self.row_count());
        }

        self.set_text_to_buffer();
    }

    pub(crate) fn set_snapshot_at_bottom(&mut self, snapshot: ScreenSnapshot) {
        self.snapshot = snapshot;
        self.viewport.reset();
        self.set_text_to_buffer();
    }

    pub(crate) fn scroll(&mut self, _x: f32, y: f32) {
        self.viewport.scroll(y, self.row_count());
        self.set_text_to_buffer();
    }

    pub(crate) fn visible_cols(&self, width: u32) -> usize {
        let content_width: u32 = Self::terminal_content_width(width);

        ((content_width as f32 / self.glyphs.cell_width).floor() as usize).max(1)
    }

    pub(crate) fn visible_rows(&self, height: u32) -> usize {
        let content_height: u32 = Self::terminal_content_height(height, self.config.line_height);
        ((content_height as f32 / self.config.line_height).floor() as usize).max(1)
    }

    pub(super) fn terminal_row_y(visible_row: usize, line_height: f32, has_tab_bar: bool) -> f32 {
        if has_tab_bar {
            Self::PADDING_Y + line_height + visible_row as f32 * line_height
        } else {
            Self::PADDING_Y + visible_row as f32 * line_height
        }
    }

    pub(super) fn display_col_count(lines: &[Vec<ScreenCell>], row: usize) -> usize {
        lines.get(row).map(|line| line.len()).unwrap_or(0).max(1)
    }

    pub(super) fn row_count(&self) -> usize {
        self.snapshot.lines.len().max(1)
    }
}

#[cfg(test)]
mod tests {
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
}
