use ::glyphon::{Attrs, Color, Family, Shaping};
use solito_terminal::{ScreenCell, ScreenSnapshot};

use crate::RendererConfig;
use crate::pipeline::rect::RectSpec;

use super::{
    glyph::GlyphonResources, resources::GlyphResources, tab_bar::TabBarSnapshot,
    viewport::ViewportState,
};

pub(crate) struct TerminalView {
    pub(crate) glyphs: GlyphResources,
    snapshot: ScreenSnapshot,
    tab_bar: TabBarSnapshot,
    viewport: ViewportState,
}

impl TerminalView {
    pub(crate) const PADDING_X: f32 = 10.0;
    pub(crate) const PADDING_Y: f32 = 10.0;
    pub(crate) const DEFAULT_CARET_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    const TAB_ACTIVE_COLOR: [u8; 4] = [255, 255, 255, 255];
    const TAB_INACTIVE_COLOR: [u8; 4] = [140, 148, 160, 255];
    const TAB_SEPARATOR_COLOR: [u8; 4] = [80, 88, 100, 255];
    const TAB_BAR_HEIGHT: f32 = RendererConfig::LINE_HEIGHT;
    const TAB_TEXT_PADDING: usize = 2;
    const TAB_GAP_CHARS: usize = 1;
    const TAB_SLANT: f32 = 10.0;
    const TAB_TOP_GLOW_HEIGHT: f32 = 1.0;
    const TAB_UNDERLINE_HEIGHT: f32 = 3.0;
    const TAB_ACTIVE_BACKGROUND: [f32; 4] = [0.118, 0.161, 0.220, 0.96];
    const TAB_INACTIVE_BACKGROUND: [f32; 4] = [0.055, 0.075, 0.118, 0.72];
    const TAB_ACTIVE_UNDERLINE: [f32; 4] = [0.125, 0.827, 0.933, 1.0];
    const TAB_ACTIVE_TOP_GLOW: [f32; 4] = [0.408, 0.878, 1.0, 0.34];

    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain: wgpu::TextureFormat,
        physical_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) -> Self {
        let glyphon: GlyphonResources =
            GlyphonResources::new(device, queue, swapchain, physical_size, scale_factor);
        let mut glyphs: GlyphResources = GlyphResources::new(glyphon);

        Self::set_text_buffer_size(&mut glyphs, physical_size.width, physical_size.height);

        Self {
            glyphs,
            snapshot: ScreenSnapshot::default(),
            tab_bar: TabBarSnapshot::default(),
            viewport: ViewportState::new(Self::terminal_content_height(physical_size.height)),
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32, snapshot: ScreenSnapshot) {
        self.snapshot = snapshot;
        Self::set_text_buffer_size(&mut self.glyphs, width, height);
        self.viewport
            .resize(Self::terminal_content_height(height), self.row_count());
        self.set_text_to_buffer();
    }

    pub(crate) fn set_tab_bar(&mut self, tab_bar: TabBarSnapshot) {
        self.tab_bar = tab_bar;
        self.set_text_to_buffer();
    }

    pub(crate) fn set_snapshot(&mut self, snapshot: ScreenSnapshot) {
        self.viewport.reset();
        self.snapshot = snapshot;
        self.set_text_to_buffer();
    }

    pub(crate) fn scroll(&mut self, _x: f32, y: f32) {
        self.viewport.scroll(y, self.row_count());
        self.set_text_to_buffer();
    }

    pub(crate) fn caret_rect(&mut self) -> (f32, f32, f32, f32) {
        let row_count: usize = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end): (usize, usize) = self.viewport.visible_range(row_count);

        if self.snapshot.cursor_row < start || self.snapshot.cursor_row >= end {
            return (Self::PADDING_X, Self::terminal_origin_y(), 0.0, 0.0);
        }

        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system).max(1.0);
        let visible_row: usize = self.snapshot.cursor_row - start;

        let caret_x: f32 = Self::PADDING_X + self.snapshot.cursor_col as f32 * cell_width;
        let caret_y: f32 = if self.tab_bar.titles().len() <= 1 {
            Self::PADDING_Y + visible_row as f32 * RendererConfig::LINE_HEIGHT
        } else {
            Self::PADDING_Y
                + RendererConfig::LINE_HEIGHT
                + visible_row as f32 * RendererConfig::LINE_HEIGHT
        };

        (caret_x, caret_y, cell_width, RendererConfig::LINE_HEIGHT)
    }

    pub(crate) fn caret_color(&self) -> [f32; 4] {
        self.snapshot
            .cursor_color
            .map(Self::rgba_to_f32)
            .unwrap_or(Self::DEFAULT_CARET_COLOR)
    }

    pub(crate) fn tab_bar_rects(&mut self, width: u32) -> Vec<RectSpec> {
        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system).max(1.0);

        Self::tab_bar_rects_for(&self.tab_bar, width, cell_width)
    }

    fn set_text_to_buffer(&mut self) {
        let spans: Vec<(String, Attrs<'static>)> = self.visible_text_spans();
        let attrs: Attrs<'static> = Self::text_attrs(None);

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

    fn visible_text_spans(&mut self) -> Vec<(String, Attrs<'static>)> {
        let mut spans: Vec<(String, Attrs<'static>)> = Self::tab_bar_spans(&self.tab_bar);
        let has_tab_bar: bool = !spans.is_empty();

        if self.snapshot.lines.is_empty() {
            return spans;
        }

        let row_count: usize = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end): (usize, usize) = self.viewport.visible_range(row_count);

        if has_tab_bar {
            spans.push(("\n".to_string(), Self::text_attrs(None)));
        }

        spans.extend(Self::text_spans_for_lines(
            &self.snapshot.lines[start..end],
            start,
            self.snapshot.cursor_row,
            self.snapshot.cursor_col,
            Self::cursor_text_color(self.caret_color()),
        ));

        spans
    }

    fn tab_bar_spans(tab_bar: &TabBarSnapshot) -> Vec<(String, Attrs<'static>)> {
        let mut spans: Vec<(String, Attrs<'static>)> = Vec::new();

        if tab_bar.titles().len() <= 1 {
            return spans;
        }

        for (index, title) in tab_bar.titles().iter().enumerate() {
            if index > 0 {
                spans.push((
                    " ".repeat(Self::TAB_GAP_CHARS),
                    Self::text_attrs(Some(Self::TAB_SEPARATOR_COLOR)),
                ));
            }

            let active: bool = index == tab_bar.active_index();
            let text: String = Self::padded_tab_title(title);

            let color: [u8; 4] = if active {
                Self::TAB_ACTIVE_COLOR
            } else {
                Self::TAB_INACTIVE_COLOR
            };

            spans.push((text, Self::text_attrs(Some(color))));
        }

        spans
    }

    fn tab_bar_rects_for(tab_bar: &TabBarSnapshot, _width: u32, cell_width: f32) -> Vec<RectSpec> {
        let mut rects: Vec<RectSpec> = Vec::new();

        if tab_bar.titles().len() <= 1 {
            return rects;
        }

        let mut x: f32 = Self::PADDING_X;
        let tab_y: f32 = 6.0;
        let tab_height: f32 = Self::TAB_BAR_HEIGHT + 2.0;

        for (index, title) in tab_bar.titles().iter().enumerate() {
            if index > 0 {
                x += cell_width * Self::TAB_GAP_CHARS as f32 + Self::TAB_SLANT;
            }

            let tab_width: f32 = Self::tab_title_width(title, cell_width);
            let slanted_width: f32 = tab_width + Self::TAB_SLANT;
            let active: bool = index == tab_bar.active_index();
            let background: [f32; 4] = if active {
                Self::TAB_ACTIVE_BACKGROUND
            } else {
                Self::TAB_INACTIVE_BACKGROUND
            };

            rects.push(RectSpec::slanted(
                x,
                tab_y,
                slanted_width,
                tab_height,
                background,
                Self::TAB_SLANT,
            ));

            if active {
                rects.push(Self::tab_strip_rect(
                    x,
                    tab_y,
                    tab_height,
                    0.0,
                    Self::TAB_TOP_GLOW_HEIGHT,
                    slanted_width,
                    Self::TAB_ACTIVE_TOP_GLOW,
                ));
                rects.push(Self::tab_strip_rect(
                    x,
                    tab_y,
                    tab_height,
                    tab_height - Self::TAB_UNDERLINE_HEIGHT,
                    Self::TAB_UNDERLINE_HEIGHT,
                    slanted_width,
                    Self::TAB_ACTIVE_UNDERLINE,
                ));
            }

            x += slanted_width;
        }

        rects
    }

    fn tab_strip_rect(
        tab_x: f32,
        tab_y: f32,
        tab_height: f32,
        strip_y: f32,
        strip_height: f32,
        width: f32,
        color: [f32; 4],
    ) -> RectSpec {
        let bottom_slant: f32 = Self::TAB_SLANT * (1.0 - (strip_y + strip_height) / tab_height);
        let strip_slant: f32 = Self::TAB_SLANT * strip_height / tab_height;

        RectSpec::slanted(
            tab_x + bottom_slant,
            tab_y + strip_y,
            width,
            strip_height,
            color,
            strip_slant,
        )
    }

    fn padded_tab_title(title: &str) -> String {
        format!(
            "{}{}{}",
            " ".repeat(Self::TAB_TEXT_PADDING),
            title,
            " ".repeat(Self::TAB_TEXT_PADDING)
        )
    }

    fn tab_title_width(title: &str, cell_width: f32) -> f32 {
        Self::padded_tab_title(title).chars().count() as f32 * cell_width
    }

    fn text_spans_for_lines(
        lines: &[Vec<ScreenCell>],
        first_row: usize,
        cursor_row: usize,
        cursor_col: usize,
        cursor_text_color: [u8; 4],
    ) -> Vec<(String, Attrs<'static>)> {
        let mut spans: Vec<(String, Attrs<'static>)> = Vec::new();
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
                    Self::push_text_span(&mut spans, &mut current_text, current_color);
                    current_color = color;
                }

                current_text.push(Self::cell_char(cell));
            }

            if line_index + 1 < lines.len() {
                current_text.push('\n');
            }
        }

        Self::push_text_span(&mut spans, &mut current_text, current_color);

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

    fn push_text_span(
        spans: &mut Vec<(String, Attrs<'static>)>,
        text: &mut String,
        color: Option<[u8; 4]>,
    ) {
        if !text.is_empty() {
            spans.push((std::mem::take(text), Self::text_attrs(color)));
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
            [0, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        }
    }

    fn text_attrs(color: Option<[u8; 4]>) -> Attrs<'static> {
        let attrs: Attrs<'static> = Attrs::new().family(Family::Name("Cascadia Mono"));

        match color {
            Some([r, g, b, a]) => attrs.color(Color::rgba(r, g, b, a)),
            None => attrs,
        }
    }

    fn rgba_to_f32([r, g, b, a]: [u8; 4]) -> [f32; 4] {
        [
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            f32::from(a) / 255.0,
        ]
    }

    fn row_count(&self) -> usize {
        self.snapshot.lines.len().max(1)
    }

    fn cell_char(cell: &ScreenCell) -> char {
        cell.ch
    }

    pub(crate) fn visible_cols(&mut self, width: u32) -> usize {
        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system).max(1.0);
        let content_width: u32 = Self::terminal_content_width(width);

        ((content_width as f32 / cell_width).floor() as usize).max(1)
    }

    pub(crate) fn visible_rows(&self, height: u32) -> usize {
        let content_height: u32 = Self::terminal_content_height(height);
        ((content_height as f32 / RendererConfig::LINE_HEIGHT).floor() as usize).max(1)
    }

    fn terminal_origin_y() -> f32 {
        Self::PADDING_Y + Self::TAB_BAR_HEIGHT
    }

    fn terminal_content_height(height: u32) -> u32 {
        height.saturating_sub(Self::TAB_BAR_HEIGHT.ceil() as u32)
    }

    fn terminal_content_width(width: u32) -> u32 {
        let horizontal_padding: u32 = (Self::PADDING_X * 2.0).ceil() as u32;

        width.saturating_sub(horizontal_padding).max(1)
    }

    fn set_text_buffer_size(glyphs: &mut GlyphResources, width: u32, height: u32) {
        glyphs.text_buffer.set_size(
            &mut glyphs.font_system,
            Some(Self::terminal_content_width(width) as f32),
            Some(Self::terminal_content_height(height) as f32),
        );
        glyphs
            .text_buffer
            .shape_until_scroll(&mut glyphs.font_system, false);
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalView;
    use crate::terminal_view::TabBarSnapshot;
    use glyphon::Color;
    use solito_terminal::ScreenCell;

    #[test]
    fn cell_char_returns_cell_character() {
        let mut cell: ScreenCell = ScreenCell::default();
        cell.ch = 'A';

        assert_eq!(TerminalView::cell_char(&cell), 'A');
    }

    #[test]
    fn text_attrs_include_foreground_color() {
        let attrs = TerminalView::text_attrs(Some([1, 2, 3, 4]));

        assert_eq!(attrs.color_opt, Some(Color::rgba(1, 2, 3, 4)));
    }

    #[test]
    fn rgba_to_f32_normalizes_color_channels() {
        assert_eq!(
            TerminalView::rgba_to_f32([0, 128, 255, 255]),
            [0.0, 128.0 / 255.0, 1.0, 1.0]
        );
    }

    #[test]
    fn caret_color_defaults_to_white() {
        assert_eq!(TerminalView::DEFAULT_CARET_COLOR, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn cursor_text_color_contrasts_with_caret_color() {
        assert_eq!(
            TerminalView::cursor_text_color([1.0, 1.0, 1.0, 1.0]),
            [0, 0, 0, 255]
        );
        assert_eq!(
            TerminalView::cursor_text_color([0.0, 0.0, 0.0, 1.0]),
            [255, 255, 255, 255]
        );
    }

    #[test]
    fn text_spans_override_cursor_cell_color() {
        let mut cell: ScreenCell = ScreenCell::default();
        cell.ch = 'A';

        let spans = TerminalView::text_spans_for_lines(&[vec![cell]], 0, 0, 0, [0, 0, 0, 255]);

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].0, "A");
        assert_eq!(spans[0].1.color_opt, Some(Color::rgba(0, 0, 0, 255)));
    }

    #[test]
    fn tab_bar_spans_mark_active_tab() {
        let snapshot: TabBarSnapshot =
            TabBarSnapshot::new(vec!["Tab 1".to_string(), "Tab 2".to_string()], 0);
        let spans = TerminalView::tab_bar_spans(&snapshot);

        assert_eq!(spans[0].0, "  Tab 1  ");
        assert_eq!(spans[0].1.color_opt, Some(Color::rgba(255, 255, 255, 255)));
        assert_eq!(spans[1].0, " ");
        assert_eq!(spans[2].0, "  Tab 2  ");
        assert_eq!(spans[2].1.color_opt, Some(Color::rgba(140, 148, 160, 255)));
    }

    #[test]
    fn tab_bar_rects_include_background_tabs_and_active_accents() {
        let snapshot: TabBarSnapshot =
            TabBarSnapshot::new(vec!["Tab 1".to_string(), "Tab 2".to_string()], 0);
        let rects = TerminalView::tab_bar_rects_for(&snapshot, 220, 10.0);

        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0].x, TerminalView::PADDING_X);
        assert_eq!(rects[0].width, 100.0);
        assert_eq!(rects[0].height, TerminalView::TAB_BAR_HEIGHT + 2.0);
        assert_eq!(rects[0].color, TerminalView::TAB_ACTIVE_BACKGROUND);
        assert_eq!(rects[0].slant, TerminalView::TAB_SLANT);
        assert_eq!(rects[2].height, TerminalView::TAB_UNDERLINE_HEIGHT);
        assert_eq!(rects[2].color, TerminalView::TAB_ACTIVE_UNDERLINE);
        assert_eq!(
            rects[2].slant,
            TerminalView::TAB_SLANT * TerminalView::TAB_UNDERLINE_HEIGHT / rects[0].height
        );
        assert_eq!(rects[3].x, 130.0);
        assert_eq!(rects[3].color, TerminalView::TAB_INACTIVE_BACKGROUND);
    }

    #[test]
    fn tab_bar_rects_hide_for_single_tab() {
        let snapshot: TabBarSnapshot = TabBarSnapshot::new(vec!["Tab 1".to_string()], 0);

        assert!(TerminalView::tab_bar_rects_for(&snapshot, 220, 10.0).is_empty());
    }

    #[test]
    fn visible_rows_reserve_one_row_for_tab_bar() {
        assert_eq!(TerminalView::terminal_content_height(90), 60);
    }

    #[test]
    fn terminal_content_width_reserves_horizontal_padding() {
        assert_eq!(TerminalView::terminal_content_width(100), 80);
        assert_eq!(TerminalView::terminal_content_width(10), 1);
    }
}
