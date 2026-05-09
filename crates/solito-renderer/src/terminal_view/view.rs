use ::glyphon::{Attrs, Color, Family, Shaping};
use solito_terminal::{ScreenCell, ScreenSnapshot};

use crate::RendererConfig;

use super::{glyph::GlyphonResources, resources::GlyphResources, viewport::ViewportState};

pub(crate) struct TerminalView {
    pub(crate) glyphs: GlyphResources,
    snapshot: ScreenSnapshot,
    viewport: ViewportState,
}

impl TerminalView {
    pub(crate) const PADDING_X: f32 = 10.0;
    pub(crate) const PADDING_Y: f32 = 10.0;
    pub(crate) const DEFAULT_CARET_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain: wgpu::TextureFormat,
        physical_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) -> Self {
        let glyphon: GlyphonResources =
            GlyphonResources::new(device, queue, swapchain, physical_size, scale_factor);

        Self {
            glyphs: GlyphResources::new(glyphon),
            snapshot: ScreenSnapshot::default(),
            viewport: ViewportState::new(physical_size.height),
        }
    }

    pub(crate) fn resize(&mut self, height: u32, snapshot: ScreenSnapshot) {
        self.snapshot = snapshot;
        self.viewport.resize(height, self.row_count());
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
            return (Self::PADDING_X, Self::PADDING_Y, 0.0, 0.0);
        }

        let cell_width: f32 =
            GlyphonResources::measure_font_width(&mut self.glyphs.font_system).max(1.0);
        let visible_row: usize = self.snapshot.cursor_row - start;

        (
            Self::PADDING_X + self.snapshot.cursor_col as f32 * cell_width,
            Self::PADDING_Y + visible_row as f32 * RendererConfig::LINE_HEIGHT,
            cell_width,
            RendererConfig::LINE_HEIGHT,
        )
    }

    pub(crate) fn caret_color(&self) -> [f32; 4] {
        self.snapshot
            .cursor_color
            .map(Self::rgba_to_f32)
            .unwrap_or(Self::DEFAULT_CARET_COLOR)
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
        if self.snapshot.lines.is_empty() {
            return Vec::new();
        }

        let row_count: usize = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end): (usize, usize) = self.viewport.visible_range(row_count);

        Self::text_spans_for_lines(&self.snapshot.lines[start..end])
    }

    fn text_spans_for_lines(lines: &[Vec<ScreenCell>]) -> Vec<(String, Attrs<'static>)> {
        let mut spans: Vec<(String, Attrs<'static>)> = Vec::new();
        let mut current_text: String = String::new();
        let mut current_color: Option<[u8; 4]> = None;

        for (line_index, line) in lines.iter().enumerate() {
            for cell in line.iter().filter(|cell| !cell.is_wide_continuation) {
                let color: Option<[u8; 4]> = cell.foreground_rgba();
                if current_text.is_empty() {
                    current_color = color;
                } else if current_color != color {
                    spans.push((current_text, Self::text_attrs(current_color)));
                    current_text = String::new();
                    current_color = color;
                }

                current_text.push(Self::cell_char(cell));
            }

            if line_index + 1 < lines.len() {
                current_text.push('\n');
            }
        }

        if !current_text.is_empty() {
            spans.push((current_text, Self::text_attrs(current_color)));
        }

        spans
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
        ((width as f32 / cell_width).floor() as usize).max(1)
    }

    pub(crate) fn visible_rows(&self, height: u32) -> usize {
        ((height as f32 / RendererConfig::LINE_HEIGHT).floor() as usize).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalView;
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
}
