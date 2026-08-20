use glyphon::{Attrs, Color, Family, Shaping};
use solito_terminal::ScreenCell;
use std::collections::HashMap;

use crate::{RendererConfig, terminal_view::glyph::GlyphonResources, util::color::ThemeColor};

use super::TerminalView;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct GridTextStyle {
    color: Option<[u8; 4]>,
    letter_spacing: f32,
}

struct GridMetrics<'a> {
    cell_width: f32,
    font_size: f32,
    glyph_widths: &'a HashMap<char, f32>,
}

impl TerminalView {
    pub(super) fn set_text_buffer_size(
        glyphs: &mut GlyphonResources,
        width: u32,
        height: u32,
        config: &RendererConfig,
    ) {
        glyphs.text_buffer.set_size(
            Some(Self::terminal_content_width(width) as f32),
            Some(Self::terminal_content_height(height, config.line_height) as f32),
        );
        glyphs
            .text_buffer
            .shape_until_scroll(&mut glyphs.font_system, false);
    }

    pub(super) fn terminal_content_height(height: u32, line_height: f32) -> u32 {
        height.saturating_sub(line_height.ceil() as u32)
    }

    pub(super) fn terminal_content_width(width: u32) -> u32 {
        let horizontal_padding = (Self::PADDING_X * 2.0).ceil() as u32;

        width.saturating_sub(horizontal_padding).max(1)
    }

    pub(super) fn mark_text_buffer_dirty(&mut self) {
        self.text_buffer_dirty = true;
    }

    pub(crate) fn rebuild_text_buffer_if_dirty(&mut self) {
        if !self.text_buffer_dirty {
            return;
        }

        self.set_text_to_buffer();
        self.text_buffer_dirty = false;
    }

    fn set_text_to_buffer(&mut self) {
        let font_family = self.config.font_family.clone();
        let spans = self.visible_text_spans(font_family.as_str());
        let attrs = Self::text_attrs(None, font_family.as_str());

        self.glyphs.text_buffer.set_rich_text(
            spans
                .iter()
                .map(|(text, attrs)| (text.as_str(), attrs.clone())),
            &attrs,
            Shaping::Advanced,
            None,
        );
        // cosmic-text 0.19 no longer shapes implicitly when text is replaced.
        // Glyphon can only prepare glyphs after the buffer has layout runs.
        self.glyphs
            .text_buffer
            .shape_until_scroll(&mut self.glyphs.font_system, false);
    }

    pub(super) fn text_attrs<'a>(color: Option<[u8; 4]>, font_family: &'a str) -> Attrs<'a> {
        let attrs = Attrs::new().family(Family::Name(font_family));

        match color {
            Some([r, g, b, a]) => attrs.color(Color::rgba(r, g, b, a)),
            None => attrs,
        }
    }

    fn visible_text_spans<'a>(&mut self, font_family: &'a str) -> Vec<(String, Attrs<'a>)> {
        let cell_width = self.glyphs.cell_width;
        let mut spans = Self::tab_bar_spans_for(&self.tab_bar, font_family, cell_width);
        let has_tab_bar = !spans.is_empty();

        if self.snapshot.lines.is_empty() {
            return spans;
        }

        let row_count = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end) = self.viewport.visible_range(row_count);

        if has_tab_bar {
            spans.push(("\n".to_string(), Self::text_attrs(None, font_family)));
        }

        self.ensure_glyph_widths(start, end);
        spans.extend(Self::text_spans_for_lines(
            &self.snapshot.lines[start..end],
            start,
            self.snapshot.cursor_row,
            self.snapshot.cursor_col,
            Self::cursor_text_color(self.caret_color()),
            font_family,
            &GridMetrics {
                cell_width,
                font_size: self.config.font_size,
                glyph_widths: &self.glyphs.glyph_widths,
            },
        ));

        spans
    }

    fn ensure_glyph_widths(&mut self, start: usize, end: usize) {
        let chars = self.snapshot.lines[start..end]
            .iter()
            .flat_map(|line| line.iter().map(|cell| cell.ch))
            .collect::<Vec<_>>();

        for ch in chars {
            if self.glyphs.glyph_widths.contains_key(&ch) {
                continue;
            }

            let mut encoded: [u8; 4] = [0; 4];
            let text = ch.encode_utf8(&mut encoded);
            let width = GlyphonResources::measure_text_width(
                &mut self.glyphs.font_system,
                &self.config,
                text,
            );
            self.glyphs.glyph_widths.insert(ch, width);
        }
    }

    fn cursor_text_color(caret_color: [f32; 4]) -> [u8; 4] {
        const RED_LUMINANCE_WEIGHT: f32 = 0.2126;
        const GREEN_LUMINANCE_WEIGHT: f32 = 0.7152;
        const BLUE_LUMINANCE_WEIGHT: f32 = 0.0722;
        const LIGHT_BACKGROUND_THRESHOLD: f32 = 0.5;

        let luminance = RED_LUMINANCE_WEIGHT * caret_color[0]
            + GREEN_LUMINANCE_WEIGHT * caret_color[1]
            + BLUE_LUMINANCE_WEIGHT * caret_color[2];

        if luminance > LIGHT_BACKGROUND_THRESHOLD {
            ThemeColor::BLACK
        } else {
            ThemeColor::WHITE
        }
    }

    fn text_spans_for_lines<'a>(
        lines: &[Vec<ScreenCell>],
        first_row: usize,
        cursor_row: usize,
        cursor_col: usize,
        cursor_text_color: [u8; 4],
        font_family: &'a str,
        grid: &GridMetrics<'_>,
    ) -> Vec<(String, Attrs<'a>)> {
        let mut spans = Vec::new();
        let mut current_text = String::new();
        let mut current_style = GridTextStyle::default();

        for (line_index, line) in lines.iter().enumerate() {
            let absolute_row = first_row + line_index;
            for (cell_col, cell) in line.iter().enumerate() {
                let color: Option<[u8; 4]> = Self::cell_text_color(
                    absolute_row,
                    cell_col,
                    cursor_row,
                    cursor_col,
                    cursor_text_color,
                    cell,
                );
                let glyph_width = grid
                    .glyph_widths
                    .get(&cell.ch)
                    .copied()
                    .unwrap_or(grid.cell_width);
                let style = GridTextStyle {
                    color,
                    letter_spacing: Self::grid_letter_spacing(
                        grid.cell_width,
                        glyph_width,
                        grid.font_size,
                    ),
                };

                if current_text.is_empty() {
                    current_style = style;
                } else if current_style != style {
                    Self::push_text_span(&mut spans, &mut current_text, current_style, font_family);
                    current_style = style;
                }

                current_text.push(cell.ch);
            }

            if line_index + 1 < lines.len() {
                current_text.push('\n');
            }
        }

        Self::push_text_span(&mut spans, &mut current_text, current_style, font_family);

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

    fn push_text_span<'a>(
        spans: &mut Vec<(String, Attrs<'a>)>,
        text: &mut String,
        style: GridTextStyle,
        font_family: &'a str,
    ) {
        if !text.is_empty() {
            let attrs = Self::text_attrs(style.color, font_family);
            let attrs = if style.letter_spacing == 0.0 {
                attrs
            } else {
                attrs.letter_spacing(style.letter_spacing)
            };
            spans.push((std::mem::take(text), attrs));
        }
    }

    fn grid_letter_spacing(cell_width: f32, glyph_width: f32, font_size: f32) -> f32 {
        let correction = cell_width - glyph_width;
        if correction.abs() < 0.01 {
            0.0
        } else {
            correction / font_size
        }
    }
}

#[cfg(test)]
mod tests {
    use glyphon::{Buffer, Color, FontSystem, Metrics, Shaping, Wrap};
    use solito_terminal::ScreenCell;
    use std::collections::HashMap;

    use crate::{
        RendererConfig,
        terminal_view::{TerminalView, glyph::GlyphonResources},
        util::color::ThemeColor,
    };

    use super::GridMetrics;

    fn color([r, g, b, a]: [u8; 4]) -> Color {
        Color::rgba(r, g, b, a)
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
        let mut cell = ScreenCell::default();
        cell.ch = 'A';
        let glyph_widths = HashMap::from([('A', 10.0)]);

        let spans = TerminalView::text_spans_for_lines(
            &[vec![cell]],
            0,
            0,
            0,
            ThemeColor::BLACK,
            RendererConfig::DEFAULT_FONT_FAMILY,
            &GridMetrics {
                cell_width: 10.0,
                font_size: RendererConfig::DEFAULT_FONT_SIZE,
                glyph_widths: &glyph_widths,
            },
        );

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].0, "A");
        assert_eq!(spans[0].1.color_opt, Some(color(ThemeColor::BLACK)));
    }

    #[test]
    fn text_attrs_include_foreground_color() {
        let attrs =
            TerminalView::text_attrs(Some([1, 2, 3, 4]), RendererConfig::DEFAULT_FONT_FAMILY);

        assert_eq!(attrs.color_opt, Some(Color::rgba(1, 2, 3, 4)));
    }

    #[test]
    fn text_spans_preserve_wide_continuation_as_grid_cell() {
        let mut ascii_a = ScreenCell::default();
        ascii_a.ch = 'A';
        let mut wide = ScreenCell::default();
        wide.ch = 'あ';
        let mut continuation = ScreenCell::default();
        continuation.ch = ' ';
        continuation.is_wide_continuation = true;
        let mut ascii_b = ScreenCell::default();
        ascii_b.ch = 'B';
        let glyph_widths = HashMap::from([('A', 10.0), ('あ', 10.0), (' ', 10.0), ('B', 10.0)]);

        let spans = TerminalView::text_spans_for_lines(
            &[vec![ascii_a, wide, continuation, ascii_b]],
            0,
            usize::MAX,
            usize::MAX,
            ThemeColor::BLACK,
            RendererConfig::DEFAULT_FONT_FAMILY,
            &GridMetrics {
                cell_width: 10.0,
                font_size: RendererConfig::DEFAULT_FONT_SIZE,
                glyph_widths: &glyph_widths,
            },
        );
        let text = spans
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>();

        assert_eq!(text, "Aあ B");
    }

    #[test]
    fn grid_spacing_places_ascii_after_wide_character_at_expected_column() {
        let config = RendererConfig::default();
        let mut font_system = FontSystem::new();
        let cell_width = GlyphonResources::measure_font_width(&mut font_system, &config);
        let mut glyph_widths = HashMap::new();

        for ch in ['A', 'あ', ' ', 'B'] {
            let mut encoded: [u8; 4] = [0; 4];
            let text = ch.encode_utf8(&mut encoded);
            glyph_widths.insert(
                ch,
                GlyphonResources::measure_text_width(&mut font_system, &config, text),
            );
        }

        let mut ascii_a = ScreenCell::default();
        ascii_a.ch = 'A';
        let mut wide = ScreenCell::default();
        wide.ch = 'あ';
        let mut continuation = ScreenCell::default();
        continuation.ch = ' ';
        continuation.is_wide_continuation = true;
        let mut ascii_b = ScreenCell::default();
        ascii_b.ch = 'B';

        let spans = TerminalView::text_spans_for_lines(
            &[vec![ascii_a, wide, continuation, ascii_b]],
            0,
            usize::MAX,
            usize::MAX,
            ThemeColor::BLACK,
            config.font_family.as_str(),
            &GridMetrics {
                cell_width,
                font_size: config.font_size,
                glyph_widths: &glyph_widths,
            },
        );
        let text = spans
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>();
        let b_start = text.find('B').expect("B must be present");
        let attrs = TerminalView::text_attrs(None, config.font_family.as_str());
        let mut buffer = Buffer::new(
            &mut font_system,
            Metrics::new(config.font_size, config.line_height),
        );
        buffer.set_size(Some(1000.0), Some(100.0));
        buffer.set_wrap(Wrap::None);
        buffer.set_rich_text(
            spans
                .iter()
                .map(|(text, attrs)| (text.as_str(), attrs.clone())),
            &attrs,
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut font_system, false);

        let run = buffer.layout_runs().next().expect("layout run must exist");
        let b = run
            .glyphs
            .iter()
            .find(|glyph| glyph.start == b_start)
            .expect("B glyph must exist");
        let expected_x = 3.0 * cell_width;

        assert!(
            (b.x - expected_x).abs() < 0.05,
            "B x={} expected={expected_x} cell_width={cell_width}",
            b.x
        );
    }
}
