use glyphon::{Attrs, Color, Family, FontSystem, Shaping};
use solito_terminal::{ScreenCell, ScreenSnapshot};

use crate::RendererConfig;
use crate::util::color::ThemeColor;

use super::{
    copy_mode::CopyModeSnapshot, glyph::GlyphonResources, resources::GlyphResources,
    tab_bar::TabBarSnapshot, viewport::ViewportState,
};

pub(crate) struct TerminalView {
    pub(crate) glyphs: GlyphResources,
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
        let glyphon: GlyphonResources = GlyphonResources::new(
            device,
            queue,
            swapchain,
            physical_size,
            scale_factor,
            &config,
        );
        let mut glyphs: GlyphResources = GlyphResources::new(glyphon);

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
        let mut font_system = FontSystem::new();
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

    pub(super) fn set_text_to_buffer(&mut self) {
        let font_family: String = self.config.font_family.clone();
        let spans: Vec<(String, Attrs<'_>)> = self.visible_text_spans(font_family.as_str());
        let attrs: Attrs<'_> = Self::text_attrs(None, font_family.as_str());

        self.glyphs.text_buffer.set_rich_text(
            &mut self.glyphs.font_system,
            spans
                .iter()
                .map(|(text, attrs)| (text.as_str(), attrs.clone())),
            &attrs,
            Shaping::Advanced,
            None,
        );
    }

    fn visible_text_spans<'a>(&mut self, font_family: &'a str) -> Vec<(String, Attrs<'a>)> {
        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system, &self.config)
                .max(1.0);
        let mut spans: Vec<(String, Attrs<'a>)> =
            Self::tab_bar_spans_for(&self.tab_bar, font_family, cell_width);
        let has_tab_bar: bool = !spans.is_empty();

        if self.snapshot.lines.is_empty() {
            return spans;
        }

        let row_count: usize = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end): (usize, usize) = self.viewport.visible_range(row_count);

        if has_tab_bar {
            spans.push(("\n".to_string(), Self::text_attrs(None, font_family)));
        }

        spans.extend(Self::text_spans_for_lines(
            &self.snapshot.lines[start..end],
            start,
            self.snapshot.cursor_row,
            self.snapshot.cursor_col,
            Self::cursor_text_color(self.caret_color()),
            font_family,
        ));

        spans
    }

    pub(super) fn display_col_count(lines: &[Vec<ScreenCell>], row: usize) -> usize {
        lines.get(row).map(|line| line.len()).unwrap_or(0).max(1)
    }

    fn text_spans_for_lines<'a>(
        lines: &[Vec<ScreenCell>],
        first_row: usize,
        cursor_row: usize,
        cursor_col: usize,
        cursor_text_color: [u8; 4],
        font_family: &'a str,
    ) -> Vec<(String, Attrs<'a>)> {
        let mut spans: Vec<(String, Attrs<'a>)> = Vec::new();
        let mut current_text: String = String::new();
        let mut current_color: Option<[u8; 4]> = None;

        for (line_index, line) in lines.iter().enumerate() {
            let absolute_row: usize = first_row + line_index;
            for (cell_col, cell) in line.iter().enumerate() {
                if cell.is_wide_continuation {
                    continue;
                }

                let color: Option<[u8; 4]> = Self::cell_text_color(
                    absolute_row,
                    cell_col,
                    cursor_row,
                    cursor_col,
                    cursor_text_color,
                    cell,
                );

                if current_text.is_empty() {
                    current_color = color;
                } else if Self::should_start_new_span(&current_text, current_color, color) {
                    Self::push_text_span(&mut spans, &mut current_text, current_color, font_family);
                    current_color = color;
                }

                current_text.push(Self::cell_char(cell));
            }

            if line_index + 1 < lines.len() {
                current_text.push('\n');
            }
        }

        Self::push_text_span(&mut spans, &mut current_text, current_color, font_family);

        spans
    }

    fn cell_text_color(
        absolute_row: usize,
        cell_col: usize,
        cursor_row: usize,
        cursor_col: usize,
        cursor_text_color: [u8; 4],
        cell: &ScreenCell,
    ) -> Option<[u8; 4]> {
        if absolute_row == cursor_row && cell_col == cursor_col {
            Some(cursor_text_color)
        } else {
            cell.foreground_rgba()
        }
    }

    fn should_start_new_span(
        current_text: &str,
        current_color: Option<[u8; 4]>,
        next_color: Option<[u8; 4]>,
    ) -> bool {
        !current_text.is_empty() && current_color != next_color
    }

    fn push_text_span<'a>(
        spans: &mut Vec<(String, Attrs<'a>)>,
        text: &mut String,
        color: Option<[u8; 4]>,
        font_family: &'a str,
    ) {
        if !text.is_empty() {
            spans.push((std::mem::take(text), Self::text_attrs(color, font_family)));
        }
    }

    fn cursor_text_color(caret_color: [f32; 4]) -> [u8; 4] {
        const RED_LUMINANCE_WEIGHT: f32 = 0.2126;
        const GREEN_LUMINANCE_WEIGHT: f32 = 0.7152;
        const BLUE_LUMINANCE_WEIGHT: f32 = 0.0722;
        const LIGHT_BACKGROUND_THRESHOLD: f32 = 0.5;

        let luminance: f32 = RED_LUMINANCE_WEIGHT * caret_color[0]
            + GREEN_LUMINANCE_WEIGHT * caret_color[1]
            + BLUE_LUMINANCE_WEIGHT * caret_color[2];

        if luminance > LIGHT_BACKGROUND_THRESHOLD {
            ThemeColor::BLACK
        } else {
            ThemeColor::WHITE
        }
    }

    pub(super) fn text_attrs<'a>(color: Option<[u8; 4]>, font_family: &'a str) -> Attrs<'a> {
        let attrs: Attrs<'a> = Attrs::new().family(Family::Name(font_family));

        match color {
            Some([r, g, b, a]) => attrs.color(Color::rgba(r, g, b, a)),
            None => attrs,
        }
    }

    pub(super) fn row_count(&self) -> usize {
        self.snapshot.lines.len().max(1)
    }

    fn cell_char(cell: &ScreenCell) -> char {
        cell.ch
    }

    pub(crate) fn visible_cols(&mut self, width: u32) -> usize {
        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system, &self.config)
                .max(1.0);
        let content_width: u32 = Self::terminal_content_width(width);

        ((content_width as f32 / cell_width).floor() as usize).max(1)
    }

    pub(crate) fn visible_rows(&self, height: u32) -> usize {
        let content_height: u32 = Self::terminal_content_height(height, self.config.line_height);
        ((content_height as f32 / self.config.line_height).floor() as usize).max(1)
    }

    pub(super) fn terminal_origin_y(&self) -> f32 {
        Self::terminal_origin_y_for(self.config.line_height)
    }

    fn terminal_origin_y_for(line_height: f32) -> f32 {
        Self::PADDING_Y + Self::tab_bar_height_for(line_height)
    }

    pub(super) fn terminal_row_y(visible_row: usize, line_height: f32, has_tab_bar: bool) -> f32 {
        if has_tab_bar {
            Self::PADDING_Y + line_height + visible_row as f32 * line_height
        } else {
            Self::PADDING_Y + visible_row as f32 * line_height
        }
    }

    fn terminal_content_height(height: u32, line_height: f32) -> u32 {
        height.saturating_sub(Self::tab_bar_height_for(line_height).ceil() as u32)
    }

    fn terminal_content_width(width: u32) -> u32 {
        let horizontal_padding: u32 = (Self::PADDING_X * 2.0).ceil() as u32;

        width.saturating_sub(horizontal_padding).max(1)
    }

    fn set_text_buffer_size(
        glyphs: &mut GlyphResources,
        width: u32,
        height: u32,
        config: &RendererConfig,
    ) {
        glyphs.text_buffer.set_size(
            &mut glyphs.font_system,
            Some(Self::terminal_content_width(width) as f32),
            Some(Self::terminal_content_height(height, config.line_height) as f32),
        );
        glyphs
            .text_buffer
            .shape_until_scroll(&mut glyphs.font_system, false);
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalView;
    use crate::util::color::ThemeColor;
    use crate::{RendererConfig, util};
    use glyphon::Color;
    use solito_terminal::ScreenCell;

    fn color([r, g, b, a]: [u8; 4]) -> Color {
        Color::rgba(r, g, b, a)
    }

    #[test]
    fn cell_char_returns_cell_character() {
        let mut cell: ScreenCell = ScreenCell::default();
        cell.ch = 'A';

        assert_eq!(TerminalView::cell_char(&cell), 'A');
    }

    #[test]
    fn text_attrs_include_foreground_color() {
        let attrs =
            TerminalView::text_attrs(Some([1, 2, 3, 4]), RendererConfig::DEFAULT_FONT_FAMILY);

        assert_eq!(attrs.color_opt, Some(Color::rgba(1, 2, 3, 4)));
    }

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
    fn cursor_text_color_contrasts_with_caret_color() {
        assert_eq!(
            TerminalView::cursor_text_color([1.0, 1.0, 1.0, 1.0]),
            ThemeColor::BLACK
        );
        assert_eq!(
            TerminalView::cursor_text_color([0.0, 0.0, 0.0, 1.0]),
            ThemeColor::WHITE
        );
    }

    #[test]
    fn text_spans_override_cursor_cell_color() {
        let mut cell: ScreenCell = ScreenCell::default();
        cell.ch = 'A';

        let spans = TerminalView::text_spans_for_lines(
            &[vec![cell]],
            0,
            0,
            0,
            ThemeColor::BLACK,
            RendererConfig::DEFAULT_FONT_FAMILY,
        );

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].0, "A");
        assert_eq!(spans[0].1.color_opt, Some(color(ThemeColor::BLACK)));
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
